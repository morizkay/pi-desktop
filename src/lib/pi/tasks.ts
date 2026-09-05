export const taskStatuses = ['Planned', 'In progress', 'Needs review', 'Completed'] as const;
export type TaskStatus = typeof taskStatuses[number];
export type LocalTask = { id:string; title:string; workspace:string; status:TaskStatus; sessionPath?:string; updatedAt:string };
export type WorkspaceData = { workspaces:string[]; tasks:LocalTask[] };
export const workspaceKey = 'pi-console-workspaces-v1';
export function parseWorkspaceData(raw: string | null): WorkspaceData {
  if (!raw) return {workspaces:[],tasks:[]};
  const v = JSON.parse(raw);
  if (!v || !Array.isArray(v.workspaces) || !v.workspaces.every((p:unknown)=>typeof p==='string') || !Array.isArray(v.tasks) || !v.tasks.every((t:any)=> t && typeof t.id==='string' && typeof t.title==='string' && typeof t.workspace==='string' && taskStatuses.includes(t.status) && typeof t.updatedAt==='string' && Number.isFinite(Date.parse(t.updatedAt)) && (t.sessionPath===undefined || typeof t.sessionPath==='string'))) throw new Error('Saved workspace/task data is invalid; it was not overwritten. Export or repair local browser storage before editing.');
  return {workspaces:v.workspaces,tasks:v.tasks};
}
