use error_chain::error_chain;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        SyntectError(::syntect::LoadingError);
        ParseIntError(::std::num::ParseIntError);
    }
}
