//! The startup ASCII-art banner.

/// Print the ScreenerBot startup banner.
pub fn print_banner() {
    println!("\x1b[36;1;3m");
    println!(
        r#"
   ███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗███████╗██████╗ ██████╗  ██████╗ ████████╗
   ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗╚══██╔══╝
   ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║   ██║   
   ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║   ██║   
   ███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║███████╗██║  ██║██████╔╝╚██████╔╝   ██║   
   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝╚═════╝  ╚═════╝    ╚═╝   

                                             SCREENERBOT
                                ◆ Automated Solana DeFi Trading Bot ◆

                  Website: screenerbot.io           Channel: t.me/screenerbotio
                  Docs:    screenerbot.io/docs      Group:   t.me/screenerbotio_talk
                  X:       x.com/screenerbotio      Support: t.me/screenerbotio_support
"#
    );
    println!("\x1b[0m");
}
