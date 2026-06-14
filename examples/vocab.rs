//! Print the frequency-ordered learner vocabulary, as served to the app's
//! tutor mode: `cargo run --example vocab -- [limit] [offset]`.

use haqor_core::bible::Bible;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let limit: u32 = args.next().map_or(Ok(100), |a| a.parse())?;
    let offset: u32 = args.next().map_or(Ok(0), |a| a.parse())?;

    let bible = Bible::open("data")?;
    for (i, e) in bible.vocab(limit, offset)?.iter().enumerate() {
        let class = e.lexical_class.as_deref().unwrap_or("-");
        println!(
            "{:>4}  {:>5}x  {}  [{}]  root={}  morph={}  gloss={}",
            offset as usize + i + 1,
            e.occurrences,
            e.surface,
            class,
            e.root,
            e.morph,
            e.gloss,
        );
    }
    Ok(())
}
