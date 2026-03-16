# log-watch

`log-watch` is a small Rust CLI that allows `tail -f`-ing multiple files in a directory,
*including new ones*. The last part is the reason I created this crate.

My main use case is "debug by printf" where my apps under development/test will create a new
logfile for each run (e.g. `cargo run > log-$(date -Iseconds).log`).

I'll use some `bacon`/`watchexec` invocation in one terminal to run above command and
run `log-watch | grep <interesting debug logs>` in another.

`log-watch` will detect files from new runs and will automatically include them in the output.
So there's no need to restart the grep command every time, while the logs are kept in
separate files per run to allow later analysis.

## Usage

```
Usage: log-watch [OPTIONS] --watch <DIR>

Options:
  -w, --watch <DIR>      Directory or file to watch
  -e, --extension <EXT>  Extensions to filter
  -h, --help             Print help
  -V, --version          Print version
```

if no extensions are provided, all files are considered
