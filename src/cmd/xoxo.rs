use colored::Colorize;

pub fn xoxo() {
    println!(
        "Hi, I'm Miguel, and I made Boyl.
Boyl is free and open source, but if you like and/or use Boyl often,
please consider [{}] if you can.
Otherwise, feel free to say hi to me on:

    {} @mikeevmm
    {} miguel.murca+boyl{}

You can see and contribute to the source code at

    {}

Thank you for using boyl!

{}: {}",
        "buying me a coffee".green(),
        "Twitter:".dimmed(),
        "Email:".dimmed(),
        "@gmail.com".dimmed(),
        "https://github.com/mikeevmm/boyl".underline(),
        "[coffee]".dimmed(),
        "https://github.com/mikeevmm/boyl#support".green()
    );
}
