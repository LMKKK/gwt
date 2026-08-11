import type { WorktreeView } from "./types";
import { renderTable } from "./table";

export type Key = "up" | "down" | "select" | "cancel" | "unknown";

export function parseKey(input: string): Key {
  if (input === "\r" || input === "\n") return "select";
  if (input === "j" || input === "\x1b[B") return "down";
  if (input === "k" || input === "\x1b[A") return "up";
  if (input === "q" || input === "\x1b" || input === "\x03") return "cancel";
  return "unknown";
}

export function parseKeys(input: string): Key[] {
  const keys: Key[] = [];
  for (let index = 0; index < input.length;) {
    const token = input[index] === "\x1b" && input[index + 1] === "[" && index + 2 < input.length
      ? input.slice(index, index + 3)
      : input[index]!;
    keys.push(parseKey(token));
    index += token.length;
  }
  return keys;
}

function color(enabled: boolean, code: string, text: string): string {
  return enabled ? `\x1b[${code}m${text}\x1b[0m` : text;
}

export interface TuiOptions {
  input?: NodeJS.ReadStream;
  output?: NodeJS.WriteStream;
  color?: boolean;
}

export async function selectWorktree(worktrees: WorktreeView[], options: TuiOptions = {}): Promise<string | undefined> {
  if (worktrees.length === 0) return undefined;
  const input = options.input ?? process.stdin;
  const output = options.output ?? process.stdout;
  const useColor = options.color ?? !("NO_COLOR" in process.env);
  let selected = Math.max(0, worktrees.findIndex((worktree) => worktree.current));
  let renderedLines = 0;

  const render = () => {
    if (renderedLines) output.write(`\x1b[${renderedLines}F`);
    const columns = output.columns && output.columns >= 20 ? output.columns : 120;
    const lines = renderTable(worktrees, columns - 2).map((line, index) => {
      const marker = index === 0 ? "  " : index - 1 === selected ? color(useColor, "36", "> ") : "  ";
      const body = index - 1 === selected ? color(useColor, "7", line) : line;
      return `\x1b[2K${marker}${body}`;
    });
    lines.push("\x1b[2K↑/↓ or j/k move • Enter select • q cancel");
    output.write(lines.join("\n") + "\n");
    renderedLines = lines.length;
  };

  const wasRaw = input.isRaw ?? false;
  try {
    input.setRawMode?.(true);
    input.resume();
    output.write("\x1b[?25l");
    render();
    for await (const chunk of input) {
      const raw = Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
      for (const key of parseKeys(raw)) {
        if (key === "cancel") return undefined;
        if (key === "select") return worktrees[selected]?.path;
        if (key === "up") selected = (selected - 1 + worktrees.length) % worktrees.length;
        if (key === "down") selected = (selected + 1) % worktrees.length;
        if (key === "up" || key === "down") render();
      }
    }
    return undefined;
  } finally {
    input.setRawMode?.(wasRaw);
    input.pause();
    output.write("\x1b[?25h");
  }
}
