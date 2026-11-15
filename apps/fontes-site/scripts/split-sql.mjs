// Split repository migration SQL without cutting PL/pgSQL dollar bodies or
// semicolons in standard quoted strings/identifiers and SQL comments.
export function splitSql(source) {
  const statements = [];
  let part = "",
    mode = "normal",
    dollar = "",
    depth = 0;
  for (let i = 0; i < source.length; i++) {
    const c = source[i],
      next = source[i + 1];
    if (mode === "line") {
      if (c === "\n") {
        mode = "normal";
        part += "\n";
      }
      continue;
    }
    if (mode === "comment") {
      if (c === "/" && next === "*") {
        depth++;
        i++;
      } else if (c === "*" && next === "/") {
        depth--;
        i++;
        if (!depth) mode = "normal";
      }
      continue;
    }
    if (mode === "dollar") {
      if (source.startsWith(dollar, i)) {
        part += dollar;
        i += dollar.length - 1;
        mode = "normal";
      } else part += c;
      continue;
    }
    if (mode === "single" || mode === "double") {
      part += c;
      const quote = mode === "single" ? "'" : '"';
      if (c === quote) {
        if (next === quote) {
          part += next;
          i++;
        } else mode = "normal";
      }
      continue;
    }
    if (c === "-" && next === "-") {
      mode = "line";
      part += " ";
      i++;
    } else if (c === "/" && next === "*") {
      mode = "comment";
      depth = 1;
      part += " ";
      i++;
    } else if (c === "'") {
      mode = "single";
      part += c;
    } else if (c === '"') {
      mode = "double";
      part += c;
    } else if (
      c === "$" &&
      /^(\$\$|\$[A-Za-z_][A-Za-z_0-9]*\$)/.test(source.slice(i))
    ) {
      dollar = source.slice(i).match(/^(\$\$|\$[A-Za-z_][A-Za-z_0-9]*\$)/)[0];
      part += dollar;
      i += dollar.length - 1;
      mode = "dollar";
    } else if (c === ";") {
      if (part.trim()) statements.push(part.trim());
      part = "";
    } else part += c;
  }
  if (!["normal", "line"].includes(mode))
    throw new Error("Unterminated SQL quote or comment in migration.");
  if (part.trim()) statements.push(part.trim());
  return statements;
}
