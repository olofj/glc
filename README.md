# gitlab client utility

This is just a small weekend hack to write a command-line utility to
access status of a test pipeline. I wanted something to avoid bouncing
between my terminal window and a web browser when I was doing trial and
error iterations.

So, what I'm doing now is just:

 - ... edit, compile, commit ...
 - `git push -f origin HEAD:refs/heads/olofj/testbranch`
 - ... wait a few sec
 - `gcl list-pipelines`
 - `gcl list-jobs -p <pipeline from the table above>`

## Installation

```
cargo install --path .
```
(or, if you prefer to run out of the source directory:
```
cargo build
```
then
```
cargo run list-pipelines
```
... etc


## TODO

I haven't used structopts much, and I haven't been able to get it to
work quite how I'd like it to. For show-job I wanted it to behave a bit
like `tail`, with `-f` and `-<N>` for last N lines. It takes the `-<N>`
as an argument by default and complains, for example.

