[![progress-banner](https://backend.codecrafters.io/progress/shell/ebc85b9a-41a5-43f1-a04e-0b5dac9119a6)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

This is a custom shell implementation written in Rust 🦀, built as part of the ["Build Your Own Shell" Challenge](https://app.codecrafters.io/courses/shell/overview).

**Note**: If you're viewing this repo on GitHub, head over to [codecrafters.io](https://codecrafters.io) to try the challenge.

# ✨ Features

This shell supports a variety of common command-line functionalities:
-   🚀 **Command Execution**: Executes built-in commands (like `cd`, `exit`, `echo`, `pwd`, `type`) and external programs from your system's `PATH`.
-   ⌨️ **Input Editing**:
    -   Navigate text with ← and → arrow keys.
    -   Delete characters using the Backspace key.
    -   Insert characters at the current cursor position.
-   📜 **Command History**:
    -   Stores previously entered commands.
    -   Navigate through history using the ↑ and ↓ arrow keys.
-   🔮 **Tab Completion**:
    -   Suggests executables (built-ins and those in `PATH`).
    -   Autocompletes file and directory paths, including relative paths like `./` or `../`.
    -   Completes the longest common prefix for multiple matches.
    -   Displays all possible matches when a unique prefix can't be determined.
-   ቧ **Pipelines**: Chain commands together by piping the output of one to the input of another using the `|` operator.
-   ↪️ **I/O Redirection**:
    -   Redirect `stdout` with `>` (overwrite) and `>>` (append).
    -   Redirect `stderr` with `2>` (overwrite) and `2>>` (append).
    -   Redirect both `stdout` and `stderr` using `&>` (overwrite) and `&>>` (append).
-   🚦 **Signal Handling**:
    -   `Ctrl+C`: Clears the current input line.
    -   `Ctrl+D`:
        -   Exits the shell if the input line is empty.
        -   Otherwise, displays available completions (similar to a second Tab press).