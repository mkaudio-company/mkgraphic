//! LaTeX command name -> `(glyph, AtomClass)` lookup, covering Greek
//! letters and the common operators/relations an LLM's math output
//! realistically uses. Not a Cargo dependency (`phf` etc.) -- a plain
//! `match` is more than fast enough for expressions this short, and
//! keeps this module dependency-free like the rest of `support::math`.

use super::ast::AtomClass;

/// Looks up a command name (without the leading `\`, e.g. `"alpha"` not
/// `"\alpha"`) and returns its glyph + atom class, or `None` if it isn't
/// one of the symbols this table covers -- callers treat `None` as a
/// [`super::parser::MathParseError::UnknownCommand`].
pub fn lookup_command(name: &str) -> Option<(char, AtomClass)> {
    use AtomClass::{Bin, Ord, Rel};
    Some(match name {
        // Lowercase Greek.
        "alpha" => ('\u{03B1}', Ord),
        "beta" => ('\u{03B2}', Ord),
        "gamma" => ('\u{03B3}', Ord),
        "delta" => ('\u{03B4}', Ord),
        "epsilon" => ('\u{03F5}', Ord),
        "varepsilon" => ('\u{03B5}', Ord),
        "zeta" => ('\u{03B6}', Ord),
        "eta" => ('\u{03B7}', Ord),
        "theta" => ('\u{03B8}', Ord),
        "vartheta" => ('\u{03D1}', Ord),
        "iota" => ('\u{03B9}', Ord),
        "kappa" => ('\u{03BA}', Ord),
        "lambda" => ('\u{03BB}', Ord),
        "mu" => ('\u{03BC}', Ord),
        "nu" => ('\u{03BD}', Ord),
        "xi" => ('\u{03BE}', Ord),
        "pi" => ('\u{03C0}', Ord),
        "rho" => ('\u{03C1}', Ord),
        "sigma" => ('\u{03C3}', Ord),
        "tau" => ('\u{03C4}', Ord),
        "upsilon" => ('\u{03C5}', Ord),
        "phi" => ('\u{03D5}', Ord),
        "varphi" => ('\u{03C6}', Ord),
        "chi" => ('\u{03C7}', Ord),
        "psi" => ('\u{03C8}', Ord),
        "omega" => ('\u{03C9}', Ord),
        // Uppercase Greek.
        "Gamma" => ('\u{0393}', Ord),
        "Delta" => ('\u{0394}', Ord),
        "Theta" => ('\u{0398}', Ord),
        "Lambda" => ('\u{039B}', Ord),
        "Xi" => ('\u{039E}', Ord),
        "Pi" => ('\u{03A0}', Ord),
        "Sigma" => ('\u{03A3}', Ord),
        "Upsilon" => ('\u{03A5}', Ord),
        "Phi" => ('\u{03A6}', Ord),
        "Psi" => ('\u{03A8}', Ord),
        "Omega" => ('\u{03A9}', Ord),
        // Binary operators.
        "times" => ('\u{00D7}', Bin),
        "div" => ('\u{00F7}', Bin),
        "cdot" => ('\u{22C5}', Bin),
        "pm" => ('\u{00B1}', Bin),
        "mp" => ('\u{2213}', Bin),
        "ast" => ('\u{2217}', Bin),
        "circ" => ('\u{2218}', Bin),
        "oplus" => ('\u{2295}', Bin),
        "otimes" => ('\u{2297}', Bin),
        "cup" => ('\u{222A}', Bin),
        "cap" => ('\u{2229}', Bin),
        "wedge" => ('\u{2227}', Bin),
        "vee" => ('\u{2228}', Bin),
        "setminus" => ('\u{2216}', Bin),
        // Relations.
        "leq" | "le" => ('\u{2264}', Rel),
        "geq" | "ge" => ('\u{2265}', Rel),
        "neq" | "ne" => ('\u{2260}', Rel),
        "approx" => ('\u{2248}', Rel),
        "equiv" => ('\u{2261}', Rel),
        "sim" => ('\u{223C}', Rel),
        "propto" => ('\u{221D}', Rel),
        "in" => ('\u{2208}', Rel),
        "notin" => ('\u{2209}', Rel),
        "subset" => ('\u{2282}', Rel),
        "subseteq" => ('\u{2286}', Rel),
        "supset" => ('\u{2283}', Rel),
        "supseteq" => ('\u{2287}', Rel),
        "to" | "rightarrow" => ('\u{2192}', Rel),
        "leftarrow" => ('\u{2190}', Rel),
        "leftrightarrow" => ('\u{2194}', Rel),
        "Rightarrow" => ('\u{21D2}', Rel),
        "Leftarrow" => ('\u{21D0}', Rel),
        "Leftrightarrow" => ('\u{21D4}', Rel),
        "mapsto" => ('\u{21A6}', Rel),
        "perp" => ('\u{22A5}', Rel),
        "parallel" => ('\u{2225}', Rel),
        // Ordinary symbols.
        "infty" => ('\u{221E}', Ord),
        "partial" => ('\u{2202}', Ord),
        "nabla" => ('\u{2207}', Ord),
        "forall" => ('\u{2200}', Ord),
        "exists" => ('\u{2203}', Ord),
        "emptyset" | "varnothing" => ('\u{2205}', Ord),
        "hbar" => ('\u{210F}', Ord),
        "ell" => ('\u{2113}', Ord),
        "Re" => ('\u{211C}', Ord),
        "Im" => ('\u{2111}', Ord),
        "aleph" => ('\u{2135}', Ord),
        "cdots" => ('\u{22EF}', Ord),
        "ldots" | "dots" => ('\u{2026}', Ord),
        "vdots" => ('\u{22EE}', Ord),
        "ddots" => ('\u{22F1}', Ord),
        "prime" => ('\u{2032}', Ord),
        "angle" => ('\u{2220}', Ord),
        "degree" => ('\u{00B0}', Ord),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_greek_letters_resolve() {
        assert_eq!(lookup_command("omega"), Some(('\u{03C9}', AtomClass::Ord)));
        assert_eq!(lookup_command("Omega"), Some(('\u{03A9}', AtomClass::Ord)));
    }

    #[test]
    fn known_operators_and_relations_carry_the_right_class() {
        assert_eq!(lookup_command("cdot"), Some(('\u{22C5}', AtomClass::Bin)));
        assert_eq!(lookup_command("leq"), Some(('\u{2264}', AtomClass::Rel)));
    }

    #[test]
    fn unknown_command_returns_none() {
        assert_eq!(lookup_command("notarealcommand"), None);
    }
}
