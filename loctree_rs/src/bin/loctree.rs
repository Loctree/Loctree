use std::any::Any;
use std::panic;

use loctree::cli::entrypoint::{EntryOptions, run};

fn install_broken_pipe_handler() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let is_broken = <dyn Any>::downcast_ref::<&str>(payload)
            .is_some_and(|s| s.contains("Broken pipe"))
            || <dyn Any>::downcast_ref::<String>(payload)
                .is_some_and(|s| s.contains("Broken pipe"));

        if is_broken {
            std::process::exit(0);
        }

        default_hook(info);
    }));
}

fn main() -> std::io::Result<()> {
    install_broken_pipe_handler();

    run(&EntryOptions {
        binary_name: "loctree",
        deprecated: false,
        show_banner: false,
        usage: USAGE,
    })
}

const USAGE: &str = "loctree - Compatibility alias for `loct`\n\n\
`loct` is the canonical CLI command. `loctree` remains available as a quiet\n\
compatibility alias for existing scripts and muscle memory.\n\n\
Recommended:\n  \
  loct auto                      Full scan → cached artifacts\n  \
  loct slice src/foo.ts          Extract file context for AI agent\n  \
  loct report --html out.html    Generate visual HTML report\n\n\
Alias examples:\n  \
  loctree                        Same behavior as `loct auto`\n  \
  loctree slice src/foo.ts       Same behavior as `loct slice src/foo.ts`\n  \
  loctree --for-ai               Same behavior as `loct --for-ai`\n\n\
Run `loct --help` for the full command reference.\n";
