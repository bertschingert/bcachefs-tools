use clap::Subcommand;

pub mod completions;
pub mod debug;
pub mod list;
pub mod mount;
pub mod subvolume;

pub use completions::completions;
pub use debug::list_bkeys;
pub use list::list;
pub use mount::mount;
pub use subvolume::subvolume;

#[derive(clap::Parser, Debug)]
#[command(name = "bcachefs")]
pub struct Cli {
    #[command(subcommand)]
    subcommands: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    List(list::Cli),
    Mount(mount::Cli),
    Completions(completions::Cli),
    #[command(visible_aliases = ["subvol"])]
    Subvolume(subvolume::Cli),
}

// FIXME: Can be removed after bumping MSRV >= 1.77 in favor of `c""` literals
#[macro_export]
macro_rules! c_str {
    ($lit:expr) => {
        ::std::ffi::CStr::from_bytes_with_nul(concat!($lit, "\0").as_bytes())
            .unwrap()
            .as_ptr()
    };
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    #[test]
    fn check_cstr_macro() {
        let literal = c_str!("hello");

        assert_eq!(
            literal,
            CStr::from_bytes_with_nul(b"hello\0").unwrap().as_ptr()
        );
    }
}
