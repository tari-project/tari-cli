// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

#[macro_export]
macro_rules! loading {
    ( $text:literal, $call:expr ) => {{
        let mut skin = termimad::MadSkin::default();
        skin.bold.set_fg(termimad::crossterm::style::Color::Magenta);
        let mut loader =
            spinners::Spinner::new(spinners::Spinners::Dots, skin.inline($text).to_string());
        let result = match $call {
            Ok(res) => {
                loader.stop_with_symbol("✅");
                Ok(res)
            }
            Err(error) => {
                loader.stop_with_symbol("❌");
                Err(error)
            }
        };
        result
    }};
    ( $text:expr, $call:expr ) => {{
        let mut skin = termimad::MadSkin::default();
        skin.bold.set_fg(termimad::crossterm::style::Color::Magenta);
        let mut loader = spinners::Spinner::new(
            spinners::Spinners::Dots,
            skin.inline($text.as_str()).to_string(),
        );
        let result = match $call {
            Ok(res) => {
                loader.stop_with_symbol("✅");
                Ok(res)
            }
            Err(error) => {
                loader.stop_with_symbol("❌");
                Err(error)
            }
        };
        result
    }};
}

#[macro_export]
macro_rules! md_println {
    ( $text:literal, $($args:tt)* ) => {{
        let mut skin = termimad::MadSkin::default();
        skin.bold.set_fg(termimad::crossterm::style::Color::Magenta);
        skin.print_inline(format!($text, $($args)*).as_str());
        println!();
    }};
}
