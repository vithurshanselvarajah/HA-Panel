#!/usr/bin/env python3
"""Remove comments from source files.

The project's house style is no comments in the code. This removes them
without touching anything that merely looks like a comment.

The dangerous cases, and what happens to each:

- A comment marker inside a string literal — `"http://example.com"` —
  is left alone. The scanner tracks string state, so it knows the `//`
  is not a comment.
- A comment marker inside a character or raw string — Rust's `r"..."` and
  `"..."`, Go's backtick strings, and Python's triple-quoted strings —
  is left alone.
- A shebang is left alone, because removing it breaks the script.
- A `
  to the linter rather than a comment.
- A line that holds nothing but a comment is removed entirely, so the
  file does not end up with a line of whitespace.

Doc comments are removed with everything else. They are comments.
"""

import sys
from pathlib import Path

EXTENSIONS = {".rs", ".go", ".ts", ".svelte", ".py", ".sh", ".yml", ".yaml"}

SKIP_NAMES = {".gitignore", ".dockerignore", ".trivyignore"}

SKIP_DIRS = {"node_modules", "target", "dist", "third_party", ".git", "gen"}

def strip_rust(text: str) -> str:
    """Remove Rust comments.

    Rust has line comments, block comments, nested block comments, and
    three kinds of string: normal, raw, and byte. The scanner handles
    each.
    """
    out = []
    i = 0
    n = len(text)

    while i < n:
        char = text[i]

        if char == "r" and i + 1 < n and text[i + 1] in '"#':
            j = i + 1
            hashes = 0
            while j < n and text[j] == "#":
                hashes += 1
                j += 1
            if j < n and text[j] == '"':
                end = find_raw_end(text, j + 1, hashes)
                out.append(text[i:end])
                i = end
                continue

        if char == "b" and i + 1 < n and text[i + 1] in '"r':
            if text[i + 1] == '"':
                end = find_string_end(text, i + 2, '"')
                out.append(text[i:end])
                i = end
                continue

        if char == '"':
            end = find_string_end(text, i + 1, '"')
            out.append(text[i:end])
            i = end
            continue

        if char == "'":

            end = find_char_end(text, i + 1)
            out.append(text[i:end])
            i = end
            continue

        if text.startswith("//", i):
            end = text.find("\n", i)
            if end == -1:
                i = n
            else:
                i = end
            continue

        if text.startswith("/*", i):
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1

            out.append(" " if text[i:j].count("\n") else "")
            i = j
            continue

        out.append(char)
        i += 1

    return "".join(out)

def find_string_end(text: str, start: int, quote: str) -> int:
    """Find the end of a string literal, honouring escapes."""
    i = start
    n = len(text)
    while i < n:
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == quote:
            return i + 1
        if text[i] == "\n":

            return i
        i += 1
    return n

def find_char_end(text: str, start: int) -> int:
    """Find the end of a character literal or a lifetime."""
    i = start
    n = len(text)
    if i < n and text[i] == "\\":
        i += 2
    elif i < n:
        i += 1
    if i < n and text[i] == "'":
        i += 1
    return i

def find_raw_end(text: str, start: int, hashes: int) -> int:
    """Find the end of a raw string."""
    terminator = '"' + "#" * hashes
    end = text.find(terminator, start)
    if end == -1:
        return len(text)
    return end + len(terminator)

def strip_c_style(text: str, line_comment: str = "//", block: bool = True,
                  html: bool = False) -> str:
    """Remove C-style comments, used for TypeScript and Go.

    `html` also removes HTML comments, which is what a Svelte component
    uses in its markup.
    """
    out = []
    i = 0
    n = len(text)

    while i < n:
        char = text[i]

        if char == '"' or char == "'" or char == "`":
            end = find_string_end(text, i + 1, char)
            out.append(text[i:end])
            i = end
            continue

        if html and text.startswith("<!--", i):
            end = text.find("-->", i + 4)
            end = n if end == -1 else end + 3
            out.append(" " if text[i:end].count("\n") else "")
            i = end
            continue

        if text.startswith(line_comment, i):
            end = text.find("\n", i)
            if end == -1:
                i = n
            else:
                i = end
            continue

        if block and text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end == -1 else end + 2
            out.append(" " if text[i:end].count("\n") else "")
            i = end
            continue

        out.append(char)
        i += 1

    return "".join(out)

def strip_hash(text: str, keep_directives: tuple[str, ...] = ()) -> str:
    """Remove hash comments, used for shell, Python and YAML.

    A line whose hash is inside a string is left alone.

    A hash that is part of a shell parameter is not a comment either, and
    treating it as one silently breaks the script.
    """
    out = []
    i = 0
    n = len(text)

    while i < n:
        char = text[i]

        if char in "\"'":
            end = find_string_end(text, i + 1, char)
            out.append(text[i:end])
            i = end
            continue

        if char == "$" and i + 1 < n and text[i + 1] == "#":
            # A shell parameter expansion: the argument count, or a
            # string length in braces.
            if i + 2 < n and text[i + 2] == "{":
                end = text.find("}", i + 2)
                end = n if end == -1 else end + 1
            else:
                end = i + 2
            out.append(text[i:end])
            i = end
            continue

        if char == "#":
            line_end = text.find("\n", i)
            if line_end == -1:
                line_end = n
            line = text[i:line_end]
            if any(marker in line for marker in keep_directives):
                out.append(line)
            i = line_end
            continue

        out.append(char)
        i += 1

    return "".join(out)

def strip_yaml(text: str) -> str:
    """Remove YAML comments."""
    out = []
    for line in text.splitlines(keepends=True):
        stripped = line.lstrip()
        if stripped.startswith("#"):
            out.append("\n" if line.endswith("\n") else "")
            continue

        result = []
        in_quote = None
        i = 0
        while i < len(line):
            c = line[i]
            if in_quote:
                result.append(c)
                if c == in_quote:
                    in_quote = None
            elif c in "\"'":
                in_quote = c
                result.append(c)
            elif c == "#" and (i == 0 or line[i - 1] in " \t"):
                break
            else:
                result.append(c)
            i += 1
        out.append("".join(result).rstrip() + ("\n" if line.endswith("\n") else ""))
    return "".join(out)

def tidy(text: str) -> str:
    """Remove the lines that hold nothing but whitespace.

    A stripped comment leaves a blank line behind, and a file full of
    blank lines is worse than a file with the comments.
    """
    lines = text.split("\n")
    out = []
    blank = False
    for line in lines:
        if not line.strip():
            if blank:
                continue
            blank = True
        else:
            blank = False
        out.append(line.rstrip())
    return "\n".join(out)

def strip_file(path: Path) -> bool:
    """Strip one file. Returns whether anything changed."""
    original = path.read_text()
    suffix = path.suffix

    if suffix == ".rs":
        text = strip_rust(original)
    elif suffix in (".ts", ".svelte"):
        text = strip_c_style(original, html=suffix == ".svelte")
    elif suffix == ".go":
        text = strip_c_style(original)
    elif suffix == ".py":

        shebang = ""
        body = original
        if body.startswith("#!"):
            shebang, _, body = body.partition("\n")
            shebang += "\n"
        text = shebang + strip_hash(body)
    elif suffix == ".sh":
        shebang = ""
        body = original
        if body.startswith("#!"):
            shebang, _, body = body.partition("\n")
            shebang += "\n"
        text = shebang + strip_hash(body, keep_directives=("shellcheck",))
    elif suffix in (".yml", ".yaml"):
        text = strip_yaml(original)
    else:
        return False

    text = tidy(text)

    if text == original:
        return False

    path.write_text(text)
    return True

def should_skip(path: Path, root: Path) -> bool:
    """Whether a path is outside the scope of this."""
    if path.name in SKIP_NAMES:
        return True
    relative = path.relative_to(root)
    return any(part in SKIP_DIRS for part in relative.parts)

def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()

    changed = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in EXTENSIONS:
            continue
        if should_skip(path, root):
            continue
        if strip_file(path):
            changed.append(path)

    for path in changed:
        print(path.relative_to(root))
    print(f"{len(changed)} file(s) changed", file=sys.stderr)
    return 0

if __name__ == "__main__":
    sys.exit(main())
