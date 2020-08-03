// This file contains some unit tests copied from the `console` project:
// https://github.com/mitsuhiko/console
//
// The MIT License (MIT)

// Copyright (c) 2017 Armin Ronacher <armin.ronacher@active-4.com>

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

#[cfg(test)]
mod tests {
    use console::{self, style};

    use crate::ansi::{measure_text_width, truncate_str};

    #[test]
    fn test_text_width() {
        let s = style("foo")
            .red()
            .on_black()
            .bold()
            .force_styling(true)
            .to_string();
        assert_eq!(measure_text_width(&s), 3);
    }

    #[test]
    fn test_truncate_str() {
        let s = format!("foo {}", style("bar").red().force_styling(true));
        assert_eq!(
            &truncate_str(&s, 5, ""),
            &format!("foo {}", style("b").red().force_styling(true))
        );
        let s = format!("foo {}", style("bar").red().force_styling(true));
        // DED: I'm changing this test assertion: delta does not move `!` inside the styled region.
        // assert_eq!(
        //     &truncate_str(&s, 5, "!"),
        //     &format!("foo {}", style("!").red().force_styling(true))
        // );
        assert_eq!(
            &truncate_str(&s, 5, "!"),
            &format!("foo {}!", style("").red().force_styling(true))
        );
        let s = format!("foo {} baz", style("bar").red().force_styling(true));
        assert_eq!(
            &truncate_str(&s, 10, "..."),
            &format!("foo {}...", style("bar").red().force_styling(true))
        );
        let s = format!("foo {}", style("バー").red().force_styling(true));
        assert_eq!(
            &truncate_str(&s, 5, ""),
            &format!("foo {}", style("").red().force_styling(true))
        );
        let s = format!("foo {}", style("バー").red().force_styling(true));
        assert_eq!(
            &truncate_str(&s, 6, ""),
            &format!("foo {}", style("バ").red().force_styling(true))
        );
    }

    #[test]
    fn test_truncate_str_no_ansi() {
        assert_eq!(&truncate_str("foo bar", 5, ""), "foo b");
        assert_eq!(&truncate_str("foo bar", 5, "!"), "foo !");
        assert_eq!(&truncate_str("foo bar baz", 10, "..."), "foo bar...");
    }
}
