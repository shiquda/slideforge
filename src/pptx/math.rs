//! Minimal inline LaTeX (`\(...\)` in PPTD rich text) → text segments.
//!
//! Not a full TeX engine: covers the common math vocabulary (Greek letters,
//! relations, operators, `\mathbb` blackboard letters, `^`/`_` scripts with
//! `{}` groups, nested commands inside scripts). Unknown commands pass
//! through verbatim so nothing is silently dropped.

/// One output segment: literal text plus an optional baseline shift in
/// 1000ths of a percent (OOXML `a:rPr@baseline`; positive = superscript).
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub text: String,
    pub baseline: Option<i32>,
}

/// Map a LaTeX command name (without the backslash) to a Unicode symbol.
/// Returns `None` for unknown names.
fn command_symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // relations & operators
        "le" | "leq" => "≤",
        "ge" | "geq" => "≥",
        "neq" | "ne" => "≠",
        "approx" => "≈",
        "sim" => "∼",
        "simeq" => "≃",
        "propto" => "∝",
        "equiv" => "≡",
        "mid" => "∣",
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "subseteq" => "⊆",
        "supset" => "⊃",
        "cup" => "∪",
        "cap" => "∩",
        "times" => "×",
        "cdot" => "·",
        "pm" => "±",
        "mp" => "∓",
        "div" => "÷",
        "circ" => "∘",
        "bullet" => "•",
        "ast" => "∗",
        "star" => "⋆",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "infty" => "∞",
        "partial" => "∂",
        "nabla" => "∇",
        "forall" => "∀",
        "exists" => "∃",
        "neg" | "lnot" => "¬",
        "land" | "wedge" => "∧",
        "lor" | "vee" => "∨",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "mapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "oint" => "∮",
        "sqrt" => "√",
        "prime" => "′",
        "angle" => "∠",
        "perp" => "⊥",
        "parallel" => "∥",
        "emptyset" => "∅",
        "therefore" => "∴",
        "because" => "∵",
        // Greek lowercase
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ε",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "φ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        // Greek uppercase
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        // misc letter-like
        "ell" => "ℓ",
        "hbar" => "ℏ",
        "imath" => "ı",
        "jmath" => "ȷ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "aleph" => "ℵ",
        _ => return None,
    })
}

/// Blackboard-bold letters for `\mathbb{…}`; unknown letters stay as-is.
fn mathbb(letter: char) -> char {
    match letter {
        'R' => 'ℝ',
        'N' => 'ℕ',
        'Z' => 'ℤ',
        'Q' => 'ℚ',
        'C' => 'ℂ',
        'H' => 'ℍ',
        'P' => 'ℙ',
        'D' => '𝔻',
        'E' => '𝔼',
        'T' => '𝕋',
        other => other,
    }
}

fn flush(out: &mut Vec<Segment>, cur: &mut String) {
    if !cur.is_empty() {
        out.push(Segment {
            text: std::mem::take(cur),
            baseline: None,
        });
    }
}

/// After a `^`/`_`: read `{}`-grouped or single-token script content.
fn read_script(chars: &[char], i: &mut usize) -> String {
    if *i < chars.len() && chars[*i] == '{' {
        *i += 1;
        let mut depth = 1;
        let start = *i;
        while *i < chars.len() && depth > 0 {
            match chars[*i] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                *i += 1;
            }
        }
        let end = *i;
        *i += 1; // consume closing brace
        chars[start..end].iter().collect()
    } else if *i < chars.len() && chars[*i] == '\\' {
        // single command as script, e.g. `x^\infty`
        let mut j = *i + 1;
        while j < chars.len() && chars[j].is_ascii_alphabetic() {
            j += 1;
        }
        let name: String = chars[*i + 1..j].iter().collect();
        *i = j;
        command_symbol(&name).map(str::to_owned).unwrap_or(name)
    } else if *i < chars.len() {
        let ch = chars[*i];
        *i += 1;
        ch.to_string()
    } else {
        String::new()
    }
}

/// Convert one `\(...\)` body to text segments. Scripts recurse so nested
/// commands inside `^{…}` / `_{…}` still resolve.
pub fn latex_to_segments(src: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                let mut j = i + 1;
                if j < chars.len() && chars[j].is_ascii_alphabetic() {
                    while j < chars.len() && chars[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    let name: String = chars[i + 1..j].iter().collect();
                    if name == "mathbb" {
                        if j < chars.len() && chars[j] == '{' {
                            let mut k = j;
                            let group = read_script(&chars, &mut k);
                            for ch in group.chars() {
                                cur.push(mathbb(ch));
                            }
                            j = k;
                        }
                    } else if let Some(symbol) = command_symbol(&name) {
                        cur.push_str(symbol);
                    } else {
                        // Unknown command: keep it verbatim and visible.
                        cur.push('\\');
                        cur.push_str(&name);
                    }
                    i = j;
                } else {
                    // Single-character command: \, \; \: spacing, \{ \} etc.
                    match chars.get(j) {
                        Some(',') | Some(';') | Some(':') | Some(' ') => cur.push(' '),
                        Some('{') => cur.push('{'),
                        Some('}') => cur.push('}'),
                        Some('_') => cur.push('_'),
                        Some('^') => cur.push('^'),
                        Some('%') => cur.push('%'),
                        Some('&') => cur.push('&'),
                        Some('#') => cur.push('#'),
                        Some('$') => cur.push('$'),
                        Some('\\') => cur.push(' '),
                        Some(other) => {
                            cur.push('\\');
                            cur.push(*other);
                        }
                        None => {}
                    }
                    i = (i + 2).min(chars.len());
                }
            }
            c @ ('^' | '_') => {
                flush(&mut out, &mut cur);
                i += 1;
                let script = read_script(&chars, &mut i);
                let base: i32 = if c == '^' { 30000 } else { -25000 };
                for seg in latex_to_segments(&script) {
                    out.push(Segment {
                        text: seg.text,
                        baseline: Some(base + seg.baseline.unwrap_or(0)),
                    });
                }
            }
            // Bare braces group without rendering.
            '{' | '}' => i += 1,
            ch => {
                cur.push(ch);
                i += 1;
            }
        }
    }
    flush(&mut out, &mut cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(src: &str) -> String {
        latex_to_segments(src)
            .into_iter()
            .map(|s| s.text)
            .collect()
    }
    #[test]
    fn plain_and_commands() {
        assert_eq!(joined("h(s_t) \\ge 0"), "h(st) ≥ 0");
        assert_eq!(joined("A \\sim \\pi_\\theta"), "A ∼ πθ");
        assert_eq!(joined("x \\in \\mathbb{R}"), "x ∈ ℝ");
        assert_eq!(joined("a \\times b \\mid c"), "a × b ∣ c");
    }

    #[test]
    fn scripts_carry_baseline() {
        let segs = latex_to_segments("A_t^{H}");
        assert_eq!(segs[0].text, "A");
        assert_eq!(segs[1].text, "t");
        assert_eq!(segs[1].baseline, Some(-25000));
        assert_eq!(segs[2].text, "H");
        assert_eq!(segs[2].baseline, Some(30000));
    }

    #[test]
    fn commands_inside_scripts() {
        let segs = latex_to_segments("o_{\\le t}");
        assert_eq!(segs[1].text, "≤ t");
        assert_eq!(segs[1].baseline, Some(-25000));
    }

    #[test]
    fn unknown_command_passes_through() {
        assert_eq!(joined("\\frac{1}{2}"), "\\frac12");
    }
}
