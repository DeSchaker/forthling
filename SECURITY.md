# Security policy

forthling runs Forth code with the rights of the user who starts it: Forth programs can read,
write and delete files. Running untrusted Forth code that way is therefore unsafe by design, and that alone is
not a vulnerability.

For code you do not trust there is the sandbox (`Forth::sandboxed()`, `--sandbox`, or
`sandbox=True` in Python): no file access from Forth, limits on the steps, the time and
the output of each evaluation, and a limit on the space a program takes outside its Forth
memory. Getting around it is a vulnerability: reaching a file, running on after a limit
was reached, one step that takes far longer than the time limit, or memory of the host
growing past the limits. Words that a host program adds in Rust or Python
run with the rights of that program; the sandbox cannot restrict them, so a host that
offers them to untrusted code should check `Forth::permits` as the SQLite words do.

A vulnerability is also, for example, a way to make forthling read or write memory outside
its own Forth memory, run other programs, or touch files without a FILE or BLOCK word
asking for it. forthling has no `unsafe` code and no dependencies, so a memory problem
would most likely lie in Rust's standard library. A Rust panic is a bug: please report it as an issue.

## Supported versions

Only the latest commit on `main` gets fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through
[GitHub's private vulnerability reporting](https://github.com/DeSchaker/forthling/security/advisories/new)
instead, with a few lines of Forth that show the problem if you can. You will get an
answer as soon as possible, and the fix and the advisory are published together.
