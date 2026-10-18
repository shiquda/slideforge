//! Minimal inline LaTeX (`\(...\)` in PPTD rich text) → text segments.
//!
//! Not a full TeX engine: covers the common math vocabulary (Greek letters,
//! relations, operators, `\mathbb` blackboard letters, `^`/`_` scripts with
//! `{}` groups, nested commands inside scripts) with semantic font styling
//! (italic, bold, baseline shift). Unknown commands pass through verbatim
//! so nothing is silently dropped.
//!
//! # Styling Semantics
//! - **Defaults (`Auto`)**: Ordinary Latin mathematical letters (`a`..=`z`, `A`..=`Z`)
//!   are italic per character; lowercase Greek symbols are italic variables;
//!   uppercase Greek symbols are upright; digits, operators, delimiters,
//!   and relations are upright.
//! - **Explicit scoped commands**:
//!   - `\mathrm{...}`, `\text{...}`, `\operatorname{...}`: upright normal.
//!   - `\mathbf{...}`: bold upright.
//!   - `\mathit{...}`: italic normal.
//!   - `\boldsymbol{...}` (or `\bm{...}`): bold math, preserving symbol-family shape.
//!   - `\mathnormal{...}`: resets to automatic semantics within its scope.
//! - **Inheritance**: Styles propagate recursively through braces and scripts
//!   (e.g., `\mathbf{x_t}` keeps `t` bold upright, whereas `\mathbf{x}_t`
//!   treats `t` as an ordinary italic subscript).
//! - **Operator adjacency**: Standard math operators (`\sin`, `\cos`, `\log`, etc.)
//!   remain upright, so `\sin x` segments into upright `sin` followed by italic `x`.

/// One output segment: literal text plus styling (italic, bold) and an optional
/// baseline shift in 1000ths of a percent (OOXML `a:rPr@baseline`; positive = superscript).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub baseline: Option<i32>,
    pub italic: bool,
    pub bold: bool,
}

/// Active scoped style mode governing font styling flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleMode {
    /// Default mathematical typesetting rules.
    Auto,
    /// `\mathrm`, `\text`, `\operatorname`: upright normal.
    Upright,
    /// `\mathbf`, `\textbf`: bold upright.
    BoldUpright,
    /// `\mathit`, `\textit`: italic normal.
    Italic,
    /// `\boldsymbol`, `\bm`: bold math, not a blanket italic transform.
    BoldMath,
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
        "top" => "⊤",
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
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
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
        // standard math operators and functions (upright)
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "log" => "log",
        "ln" => "ln",
        "exp" => "exp",
        "max" => "max",
        "min" => "min",
        "argmax" => "argmax",
        "argmin" => "argmin",
        "arg" => "arg",
        "det" => "det",
        "dim" => "dim",
        "ker" => "ker",
        "lim" => "lim",
        "sup" => "sup",
        "inf" => "inf",
        _ => return None,
    })
}

/// Characters that use the default mathematical variable alphabet.
fn is_math_variable_char(c: char) -> bool {
    c.is_ascii_alphabetic()
        || matches!(
            c,
            'α'..='ω' | 'ϑ' | 'ϕ' | 'ϖ' | 'ϰ' | 'ϱ' | 'ϵ' | 'ℓ' | 'ℏ' | 'ı' | 'ȷ'
        )
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

/// Resolve styling `(italic, bold)` for a single literal character given the active mode.
fn resolve_char_style(ch: char, mode: StyleMode) -> (bool, bool) {
    match mode {
        StyleMode::BoldMath => (is_math_variable_char(ch), !ch.is_whitespace()),
        // Math alphabets do not turn relations or delimiters into variables.
        _ if !ch.is_alphanumeric() && !is_math_variable_char(ch) => (false, false),
        StyleMode::Upright => (false, false),
        StyleMode::BoldUpright => (false, true),
        StyleMode::Italic => (true, false),
        StyleMode::Auto => {
            if is_math_variable_char(ch) {
                (true, false)
            } else {
                // Digits, uppercase Greek, operators, delimiters, whitespace, punctuation
                (false, false)
            }
        }
    }
}

/// Resolve styling `(italic, bold)` for a command symbol or token given the active mode.
fn resolve_command_style(symbol: &str, mode: StyleMode) -> (bool, bool) {
    let mut chars = symbol.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => resolve_char_style(ch, mode),
        // Named operators retain their own upright font inside math alphabets.
        _ => (false, mode == StyleMode::BoldMath),
    }
}

/// Buffer for coalescing consecutive character outputs that share identical styling and baseline.
#[derive(Default)]
struct PendingSegment {
    text: String,
    baseline: Option<i32>,
    italic: bool,
    bold: bool,
}

impl PendingSegment {
    fn push(
        &mut self,
        text: &str,
        baseline: Option<i32>,
        italic: bool,
        bold: bool,
        out: &mut Vec<Segment>,
    ) {
        if text.is_empty() {
            return;
        }
        if self.text.is_empty() {
            self.text.push_str(text);
            self.baseline = baseline;
            self.italic = italic;
            self.bold = bold;
        } else if self.baseline == baseline && self.italic == italic && self.bold == bold {
            self.text.push_str(text);
        } else {
            self.flush(out);
            self.text.push_str(text);
            self.baseline = baseline;
            self.italic = italic;
            self.bold = bold;
        }
    }

    fn push_char(
        &mut self,
        ch: char,
        baseline: Option<i32>,
        italic: bool,
        bold: bool,
        out: &mut Vec<Segment>,
    ) {
        if self.text.is_empty() {
            self.text.push(ch);
            self.baseline = baseline;
            self.italic = italic;
            self.bold = bold;
        } else if self.baseline == baseline && self.italic == italic && self.bold == bold {
            self.text.push(ch);
        } else {
            self.flush(out);
            self.text.push(ch);
            self.baseline = baseline;
            self.italic = italic;
            self.bold = bold;
        }
    }

    fn flush(&mut self, out: &mut Vec<Segment>) {
        if !self.text.is_empty() {
            out.push(Segment {
                text: std::mem::take(&mut self.text),
                baseline: self.baseline,
                italic: self.italic,
                bold: self.bold,
            });
            self.baseline = None;
            self.italic = false;
            self.bold = false;
        }
    }
}

/// Recursive-descent token parser within a given baseline and style scope.
fn parse_tokens(
    chars: &[char],
    i: &mut usize,
    baseline: Option<i32>,
    mode: StyleMode,
    pending: &mut PendingSegment,
    out: &mut Vec<Segment>,
) {
    while *i < chars.len() {
        match chars[*i] {
            '}' => {
                // Closing brace of an outer group: return to caller so caller can consume it.
                break;
            }
            '{' => {
                // Bare grouping `{ ... }`
                *i += 1;
                parse_tokens(chars, i, baseline, mode, pending, out);
                if *i < chars.len() && chars[*i] == '}' {
                    *i += 1;
                }
            }
            c @ ('^' | '_') => {
                *i += 1;
                let shift: i32 = if c == '^' { 30000 } else { -25000 };
                let new_baseline = Some(baseline.unwrap_or(0) + shift);
                parse_script_arg(chars, i, new_baseline, mode, pending, out);
            }
            '\\' => {
                parse_backslash(chars, i, baseline, mode, pending, out);
            }
            ch => {
                *i += 1;
                let (italic, bold) = resolve_char_style(ch, mode);
                pending.push_char(ch, baseline, italic, bold, out);
            }
        }
    }
}

/// Parse one grouped or single-token argument of a script or font command.
fn parse_script_arg(
    chars: &[char],
    i: &mut usize,
    baseline: Option<i32>,
    mode: StyleMode,
    pending: &mut PendingSegment,
    out: &mut Vec<Segment>,
) {
    while *i < chars.len() && chars[*i].is_whitespace() {
        *i += 1;
    }
    if *i >= chars.len() {
        return;
    }
    if chars[*i] == '{' {
        *i += 1;
        parse_tokens(chars, i, baseline, mode, pending, out);
        if *i < chars.len() && chars[*i] == '}' {
            *i += 1;
        }
    } else if chars[*i] == '\\' {
        parse_backslash(chars, i, baseline, mode, pending, out);
    } else {
        let ch = chars[*i];
        *i += 1;
        let (italic, bold) = resolve_char_style(ch, mode);
        pending.push_char(ch, baseline, italic, bold, out);
    }
}

/// Parse a LaTeX command starting with `\`.
fn parse_backslash(
    chars: &[char],
    i: &mut usize,
    baseline: Option<i32>,
    mode: StyleMode,
    pending: &mut PendingSegment,
    out: &mut Vec<Segment>,
) {
    *i += 1; // consume `\`
    if *i >= chars.len() {
        // Trailing single `\`
        pending.push_char('\\', baseline, false, false, out);
        return;
    }

    if chars[*i].is_ascii_alphabetic() {
        let start = *i;
        while *i < chars.len() && chars[*i].is_ascii_alphabetic() {
            *i += 1;
        }
        let name: String = chars[start..*i].iter().collect();

        // 1. Blackboard bold \mathbb{...}
        if name == "mathbb" {
            parse_mathbb(chars, i, baseline, pending, out);
            return;
        }

        // 2. Scoped style commands
        let style_override = match name.as_str() {
            "mathrm" | "text" | "operatorname" => Some(StyleMode::Upright),
            "mathbf" | "textbf" => Some(StyleMode::BoldUpright),
            "mathit" | "textit" => Some(StyleMode::Italic),
            "boldsymbol" | "bm" => Some(StyleMode::BoldMath),
            "mathnormal" => Some(StyleMode::Auto),
            _ => None,
        };

        if let Some(new_mode) = style_override {
            parse_script_arg(chars, i, baseline, new_mode, pending, out);
            return;
        }

        // 3. Known command symbol or math operator
        if let Some(symbol) = command_symbol(&name) {
            let (italic, bold) = resolve_command_style(symbol, mode);
            pending.push(symbol, baseline, italic, bold, out);
            return;
        }

        // 4. Unknown command: keep verbatim and visible
        pending.push_char('\\', baseline, false, false, out);
        pending.push(&name, baseline, false, false, out);
    } else {
        // Single-character command: \, \; \: \  \{ \} \_ \^ \% \& \# \$ \\ etc.
        let ch = chars[*i];
        *i += 1;
        match ch {
            ',' | ';' | ':' | ' ' | '\\' => {
                let (italic, bold) = resolve_char_style(' ', mode);
                pending.push_char(' ', baseline, italic, bold, out);
            }
            '{' | '}' | '_' | '^' | '%' | '&' | '#' | '$' => {
                let (italic, bold) = resolve_char_style(ch, mode);
                pending.push_char(ch, baseline, italic, bold, out);
            }
            other => {
                let (italic, bold) = resolve_char_style(other, mode);
                pending.push_char('\\', baseline, italic, bold, out);
                pending.push_char(other, baseline, italic, bold, out);
            }
        }
    }
}

/// Parse `\mathbb` arguments.
fn parse_mathbb(
    chars: &[char],
    i: &mut usize,
    baseline: Option<i32>,
    pending: &mut PendingSegment,
    out: &mut Vec<Segment>,
) {
    while *i < chars.len() && chars[*i].is_whitespace() {
        *i += 1;
    }
    if *i >= chars.len() {
        return;
    }
    if chars[*i] == '{' {
        *i += 1;
        while *i < chars.len() && chars[*i] != '}' {
            let ch = chars[*i];
            *i += 1;
            let mb = mathbb(ch);
            pending.push_char(mb, baseline, false, false, out);
        }
        if *i < chars.len() && chars[*i] == '}' {
            *i += 1;
        }
    } else {
        let ch = chars[*i];
        *i += 1;
        let mb = mathbb(ch);
        pending.push_char(mb, baseline, false, false, out);
    }
}

/// Convert one `\(...\)` body to text segments with baseline shifts and font styling.
pub fn latex_to_segments(src: &str) -> Vec<Segment> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut pending = PendingSegment::default();
    let mut out = Vec::new();
    while i < chars.len() {
        if chars[i] == '}' {
            // Stray closing brace at top level: ignore safely
            i += 1;
            continue;
        }
        parse_tokens(
            &chars,
            &mut i,
            None,
            StyleMode::Auto,
            &mut pending,
            &mut out,
        );
    }
    pending.flush(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(src: &str) -> String {
        latex_to_segments(src).into_iter().map(|s| s.text).collect()
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
        assert_eq!(segs[0].text, "o");
        assert_eq!(segs[0].baseline, None);
        assert!(segs[0].italic);
        assert_eq!(joined("o_{\\le t}"), "o≤ t");
        // Relation operator is upright, variable letter is italic
        assert!(
            segs.iter()
                .any(|s| s.text.contains('≤') && !s.italic && s.baseline == Some(-25000))
        );
        assert!(
            segs.iter()
                .any(|s| s.text.contains('t') && s.italic && s.baseline == Some(-25000))
        );
    }

    #[test]
    fn unknown_command_passes_through() {
        assert_eq!(joined("\\frac{1}{2}"), "\\frac12");
    }

    #[test]
    fn default_math_semantics() {
        // Latin letters: italic variables
        let segs_var = latex_to_segments("x");
        assert_eq!(segs_var[0].text, "x");
        assert!(segs_var[0].italic);
        assert!(!segs_var[0].bold);

        // Digits: upright normal
        let segs_num = latex_to_segments("123");
        assert_eq!(segs_num[0].text, "123");
        assert!(!segs_num[0].italic);
        assert!(!segs_num[0].bold);

        // Lowercase Greek: italic variables
        let segs_greek_low = latex_to_segments("\\alpha + \\theta");
        let alpha = segs_greek_low
            .iter()
            .find(|s| s.text.contains('α'))
            .unwrap();
        assert!(alpha.italic);
        assert!(!alpha.bold);
        let theta = segs_greek_low
            .iter()
            .find(|s| s.text.contains('θ'))
            .unwrap();
        assert!(theta.italic);

        // Uppercase Greek: upright
        let segs_greek_up = latex_to_segments("\\Gamma + \\Sigma");
        let gamma = segs_greek_up.iter().find(|s| s.text.contains('Γ')).unwrap();
        assert!(!gamma.italic);
        assert!(!gamma.bold);
        let sigma = segs_greek_up.iter().find(|s| s.text.contains('Σ')).unwrap();
        assert!(!sigma.italic);
    }

    #[test]
    fn single_letter_mathrm_upright() {
        // \mathrm{i} must remain upright normal (not heuristically turned into italic)
        let segs_i_rm = latex_to_segments("\\mathrm{i}");
        assert_eq!(segs_i_rm[0].text, "i");
        assert!(!segs_i_rm[0].italic);
        assert!(!segs_i_rm[0].bold);

        // Ordinary i is italic
        let segs_i_auto = latex_to_segments("i");
        assert_eq!(segs_i_auto[0].text, "i");
        assert!(segs_i_auto[0].italic);
        assert!(!segs_i_auto[0].bold);
    }

    #[test]
    fn operator_token_adjacency_sin_x() {
        // \sin x must produce upright sin + italic x, not merged or misclassified
        let segs = latex_to_segments("\\sin x");
        let sin_seg = segs.iter().find(|s| s.text.contains("sin")).unwrap();
        assert!(!sin_seg.italic, "sin operator must be upright");
        assert!(!sin_seg.bold);

        let x_seg = segs.iter().find(|s| s.text.trim() == "x").unwrap();
        assert!(x_seg.italic, "variable x must be italic");
        assert!(!x_seg.bold);

        // Parenthesized operator formula
        let segs_paren = latex_to_segments("\\sin(x)");
        let sin_paren = segs_paren.iter().find(|s| s.text.contains("sin")).unwrap();
        assert!(!sin_paren.italic);
        let x_paren = segs_paren.iter().find(|s| s.text == "x").unwrap();
        assert!(x_paren.italic);
    }

    #[test]
    fn explicit_scoped_style_commands() {
        // \mathbf{x} -> bold upright
        let segs_bf = latex_to_segments("\\mathbf{x}");
        assert_eq!(segs_bf[0].text, "x");
        assert!(!segs_bf[0].italic);
        assert!(segs_bf[0].bold);

        // \boldsymbol{\mu} -> bold italic
        let segs_bs = latex_to_segments("\\boldsymbol{\\mu}");
        assert_eq!(segs_bs[0].text, "μ");
        assert!(segs_bs[0].italic);
        assert!(segs_bs[0].bold);

        // \mathit{x} -> italic normal
        let segs_it = latex_to_segments("\\mathit{x}");
        assert_eq!(segs_it[0].text, "x");
        assert!(segs_it[0].italic);
        assert!(!segs_it[0].bold);

        // \text{loss} and \operatorname{argmax} -> upright normal
        let segs_txt = latex_to_segments("\\text{loss}");
        assert_eq!(segs_txt[0].text, "loss");
        assert!(!segs_txt[0].italic);
        assert!(!segs_txt[0].bold);

        let segs_op = latex_to_segments("\\operatorname{argmax}");
        assert_eq!(segs_op[0].text, "argmax");
        assert!(!segs_op[0].italic);
        assert!(!segs_op[0].bold);
    }

    #[test]
    fn style_inheritance_through_scripts() {
        // Inside \mathbf, subscript inherits bold upright: \mathbf{x_t}
        let segs_in = latex_to_segments("\\mathbf{x_t}");
        let x_in = segs_in.iter().find(|s| s.text == "x").unwrap();
        assert!(!x_in.italic);
        assert!(x_in.bold);
        assert_eq!(x_in.baseline, None);
        let t_in = segs_in.iter().find(|s| s.text == "t").unwrap();
        assert!(!t_in.italic);
        assert!(t_in.bold);
        assert_eq!(t_in.baseline, Some(-25000));

        // Outside \mathbf, subscript resets to auto semantics: \mathbf{x}_t
        let segs_out = latex_to_segments("\\mathbf{x}_t");
        let x_out = segs_out.iter().find(|s| s.text == "x").unwrap();
        assert!(!x_out.italic);
        assert!(x_out.bold);
        assert_eq!(x_out.baseline, None);
        let t_out = segs_out.iter().find(|s| s.text == "t").unwrap();
        assert!(
            t_out.italic,
            "subscript outside mathbf must be italic variable"
        );
        assert!(!t_out.bold);
        assert_eq!(t_out.baseline, Some(-25000));
    }

    #[test]
    fn mathnormal_resets_scoped_semantics() {
        // \mathbf{x + \mathnormal{y}}
        let segs = latex_to_segments("\\mathbf{x + \\mathnormal{y}}");
        let x = segs.iter().find(|s| s.text.contains('x')).unwrap();
        assert!(!x.italic);
        assert!(x.bold);
        let y = segs.iter().find(|s| s.text.contains('y')).unwrap();
        assert!(
            y.italic,
            "y inside mathnormal must have auto italic semantics"
        );
        assert!(!y.bold, "y inside mathnormal must not inherit bold");
    }

    #[test]
    fn ddpm_scientific_equation_structure() {
        // p_\theta(\mathbf{x}_0): scalar function p (italic), parameter theta (subscript italic),
        // vector x (bold upright), time index 0 (subscript upright)
        let segs = latex_to_segments("p_\\theta(\\mathbf{x}_0)");
        let p = segs.iter().find(|s| s.text == "p").unwrap();
        assert!(p.italic);
        assert!(!p.bold);
        assert_eq!(p.baseline, None);

        let theta = segs.iter().find(|s| s.text == "θ").unwrap();
        assert!(theta.italic);
        assert!(!theta.bold);
        assert_eq!(theta.baseline, Some(-25000));

        let x = segs.iter().find(|s| s.text == "x").unwrap();
        assert!(!x.italic);
        assert!(x.bold);
        assert_eq!(x.baseline, None);

        let zero = segs.iter().find(|s| s.text == "0").unwrap();
        assert!(!zero.italic);
        assert!(!zero.bold);
        assert_eq!(zero.baseline, Some(-25000));
    }

    #[test]
    fn matrix_transpose_superscript() {
        // \mathbf{A}^{\mathrm{T}}: bold upright matrix A, upright normal transpose T
        let segs = latex_to_segments("\\mathbf{A}^{\\mathrm{T}}");
        let a = segs.iter().find(|s| s.text == "A").unwrap();
        assert!(!a.italic);
        assert!(a.bold);
        assert_eq!(a.baseline, None);

        let t = segs.iter().find(|s| s.text == "T").unwrap();
        assert!(!t.italic);
        assert!(!t.bold);
        assert_eq!(t.baseline, Some(30000));
    }

    #[test]
    fn letter_like_symbols_and_operators_keep_their_semantics() {
        let segments = latex_to_segments(r"\ell + \hbar + \imath + \jmath");
        for ch in ['ℓ', 'ℏ', 'ı', 'ȷ'] {
            let segment = segments.iter().find(|s| s.text.contains(ch)).unwrap();
            assert!(segment.italic, "{ch} is a variable symbol");
            assert!(!segment.bold);
        }
        let segments = latex_to_segments(r"\mathbf{x + \sin y} + \mathit{z \ge 0}");
        for token in ["sin", "+", "≥"] {
            let segment = segments.iter().find(|s| s.text.contains(token)).unwrap();
            assert!(
                !segment.italic && !segment.bold,
                "{token} is not a variable"
            );
        }
    }

    #[test]
    fn boldsymbol_preserves_default_symbol_family_shape() {
        let segments = latex_to_segments(r"\boldsymbol{\mu + \Sigma + 1 + \sin t}");
        for (ch, italic) in [
            ('μ', true),
            ('Σ', false),
            ('1', false),
            ('+', false),
            ('t', true),
        ] {
            let segment = segments.iter().find(|s| s.text.contains(ch)).unwrap();
            assert_eq!(segment.italic, italic, "{ch}");
            assert!(segment.bold, "{ch}");
        }
        let operator = segments.iter().find(|s| s.text.contains("sin")).unwrap();
        assert!(!operator.italic);
        assert!(operator.bold);
    }
}
