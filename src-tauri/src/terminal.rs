use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::{collections::VecDeque, io::{Read, Write}, sync::{Arc, Mutex, Condvar, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc}, thread, time::Duration};
use tauri::State;

const CAPACITY: usize = 256 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Default)]
struct Output { bytes: VecDeque<u8>, eof: bool }
struct Terminal {
    id: u64, master: Box<dyn MasterPty + Send>,
    input: mpsc::SyncSender<String>,
    output: Arc<(Mutex<Output>, Condvar)>, alive: Arc<AtomicBool>,
    killer: Box<dyn portable_pty::ChildKiller + Send + Sync>, pid: Option<u32>,
    reaped: mpsc::Receiver<()>, child_alive: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Terminals(Mutex<Option<Terminal>>);
#[derive(Serialize)]
pub struct Started { id:u64, cwd:String }
#[derive(Serialize)]
pub struct Poll { bytes:Vec<u8>, exited:bool }
fn size(cols:u16, rows:u16) -> Result<PtySize,String> {
    if !(2..=500).contains(&cols) || !(1..=300).contains(&rows) { return Err("Invalid terminal size".into()); }
    Ok(PtySize {cols, rows, pixel_width:0,pixel_height:0})
}
impl Terminal {
    fn close(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        self.output.1.notify_all();
        if self.child_alive.load(Ordering::SeqCst) {
        #[cfg(unix)] unsafe {
            // Only signal groups belonging to this explicitly opened PTY, not the app's group.
            let own = libc::getpgrp();
            if let Some(group) = self.master.process_group_leader() {
                if group > 1 && group != own { libc::kill(-group, libc::SIGKILL); }
            }
            if let Some(pid) = self.pid { if pid > 1 && pid as i32 != own { libc::kill(-(pid as i32), libc::SIGKILL); } }
        }
        let _ = self.killer.kill();
        }
        let _ = self.reaped.recv_timeout(Duration::from_secs(2));
    }
}
impl Drop for Terminal { fn drop(&mut self) { self.close(); } }
impl Terminals {
    pub fn close_all(&self) { if let Ok(mut slot) = self.0.lock() { slot.take(); } }
    fn start(&self,cwd:String,cols:u16,rows:u16,shell:CommandBuilder)->Result<Started,String> {
        let dimensions=size(cols,rows)?;
        let path=std::path::Path::new(&cwd).canonicalize().map_err(|e|e.to_string())?;
        if !path.is_dir(){return Err("Terminal workspace must be a directory".into());}
        let mut slot=self.0.lock().map_err(|e|e.to_string())?;
        if slot.is_some(){return Err("Close the existing terminal before starting another".into());}
        let pair=native_pty_system().openpty(dimensions).map_err(|e|e.to_string())?;
        let mut reader=pair.master.try_clone_reader().map_err(|e|e.to_string())?;
        let mut writer=pair.master.take_writer().map_err(|e|e.to_string())?;
        let mut command=shell; command.cwd(&path); command.env("TERM","xterm-256color");
        let mut child=pair.slave.spawn_command(command).map_err(|e|e.to_string())?;
        drop(pair.slave);
        let killer=child.clone_killer(); let pid=child.process_id();
        let (reap_tx,reaped)=mpsc::channel();
        let child_alive=Arc::new(AtomicBool::new(true));let child_status=child_alive.clone();
        thread::spawn(move||{let _=child.wait();child_status.store(false,Ordering::SeqCst);let _=reap_tx.send(());});
        let output=Arc::new((Mutex::new(Output::default()),Condvar::new()));
        let alive=Arc::new(AtomicBool::new(true));
        let data=output.clone();let active=alive.clone();
        thread::spawn(move||{
            let mut buf=[0u8;8192];
            while active.load(Ordering::SeqCst) {
                let Ok(n)=reader.read(&mut buf) else {break}; if n==0{break;}
                let Ok(mut out)=data.0.lock() else {break};
                while out.bytes.len()+n>CAPACITY && active.load(Ordering::SeqCst) {out=match data.1.wait(out){Ok(g)=>g,Err(_)=>return};}
                if !active.load(Ordering::SeqCst){break;}
                out.bytes.extend(&buf[..n]);
            }
            if let Ok(mut out)=data.0.lock(){out.eof=true;}
        });
        let (input,rx)=mpsc::sync_channel::<String>(16);
        let active=alive.clone();
        thread::spawn(move||{while let Ok(text)=rx.recv(){if !active.load(Ordering::SeqCst){break;}if writer.write_all(text.as_bytes()).and_then(|_|writer.flush()).is_err(){break;}}});
        let id=NEXT.fetch_add(1,Ordering::Relaxed);
        *slot=Some(Terminal{id,master:pair.master,input,output,alive,killer,pid,reaped,child_alive});
        Ok(Started{id,cwd:path.to_string_lossy().into_owned()})
    }
}
#[tauri::command]
pub async fn terminal_start(state:State<'_,Arc<Terminals>>,pi:State<'_,crate::AppState>,cols:u16,rows:u16)->Result<Started,String>{
    let terminals=state.inner().clone();let cwd=pi.get_cwd()?;
    tauri::async_runtime::spawn_blocking(move||terminals.start(cwd,cols,rows,CommandBuilder::new_default_prog())).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub fn terminal_poll(state:State<'_,Arc<Terminals>>,id:u64)->Result<Poll,String>{
    let slot=state.0.lock().map_err(|e|e.to_string())?;let term=slot.as_ref().filter(|t|t.id==id).ok_or("Terminal is closed")?;
    let mut out=term.output.0.lock().map_err(|e|e.to_string())?;
    let bytes=out.bytes.drain(..).collect();term.output.1.notify_all();
    Ok(Poll{bytes,exited:out.eof})
}
#[tauri::command]
pub fn terminal_write(state:State<'_,Arc<Terminals>>,id:u64,data:String)->Result<(),String>{
    if data.len()>16384{return Err("Terminal input chunk too large".into());}
    let slot=state.0.lock().map_err(|e|e.to_string())?;let term=slot.as_ref().filter(|t|t.id==id).ok_or("Terminal is closed")?;
    term.input.try_send(data).map_err(|_|"Terminal input is busy; input was not accepted".into())
}
#[tauri::command]
pub fn terminal_resize(state:State<'_,Arc<Terminals>>,id:u64,cols:u16,rows:u16)->Result<(),String>{
    let dimensions=size(cols,rows)?;let slot=state.0.lock().map_err(|e|e.to_string())?;
    slot.as_ref().filter(|t|t.id==id).ok_or("Terminal is closed")?.master.resize(dimensions).map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn terminal_close(state:State<'_,Arc<Terminals>>,id:u64)->Result<(),String>{
    let terminals=state.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{let mut slot=terminals.0.lock().map_err(|e|e.to_string())?;if slot.as_ref().is_some_and(|t|t.id==id){slot.take();}Ok(())}).await.map_err(|e|e.to_string())?
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn real_pty_echo_resize_close() {
        let temp=tempfile::tempdir().unwrap();let manager=Terminals::default();
        let started=manager.start(temp.path().to_string_lossy().into_owned(),80,24,CommandBuilder::new("/bin/sh")).unwrap();
        {let slot=manager.0.lock().unwrap();let term=slot.as_ref().unwrap();assert_eq!(term.id,started.id);term.master.resize(size(100,30).unwrap()).unwrap();term.input.send("printf 'pty-test-ok\\n'; exit\n".into()).unwrap();}
        let mut output=Vec::new();let mut eof=false;
        for _ in 0..100 {thread::sleep(Duration::from_millis(20));let slot=manager.0.lock().unwrap();let term=slot.as_ref().unwrap();let mut out=term.output.0.lock().unwrap();output.extend(out.bytes.drain(..));term.output.1.notify_all();if out.eof{eof=true;break;}}
        assert!(eof);assert!(String::from_utf8_lossy(&output).contains("pty-test-ok"));manager.close_all();assert!(manager.0.lock().unwrap().is_none());assert!(size(0,0).is_err());
    }
}
