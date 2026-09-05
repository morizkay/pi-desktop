import { Marked } from "marked";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import json from "highlight.js/lib/languages/json";
import bash from "highlight.js/lib/languages/bash";
import rust from "highlight.js/lib/languages/rust";
import python from "highlight.js/lib/languages/python";
import diff from "highlight.js/lib/languages/diff";
for (const [name, language] of Object.entries({
  javascript,
  typescript,
  json,
  bash,
  rust,
  python,
  diff,
}))
  hljs.registerLanguage(name, language);
const escape = (text: string) =>
  text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
const markdown = new Marked({
  breaks: true,
  renderer: {
    code({ text, lang }) {
      const language = lang?.split(/\s/)[0] || "";
      const code = hljs.getLanguage(language)
        ? hljs.highlight(text, { language }).value
        : escape(text);
      return `<pre><code>${code}</code></pre>`;
    },
  },
});
export function renderMarkdown(text: string): string {
  if (typeof window === "undefined") return escape(text);
  // Sanitize AFTER highlighting. No remote images, SVG, forms, styles, or executable URLs.
  return DOMPurify.sanitize(markdown.parse(text, { async: false }), {
    USE_PROFILES: { html: true },
    FORBID_TAGS: [
      "img",
      "form",
      "input",
      "button",
      "style",
      "iframe",
      "video",
      "audio",
    ],
    FORBID_ATTR: ["style", "id", "name", "target"],
    ALLOW_DATA_ATTR: false,
  });
}
