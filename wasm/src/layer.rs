pub(crate) struct LayerIdentity {
    pub kind: &'static str,
    pub label: &'static str,
    pub color: &'static str,
    pub is_drill: bool,
}

pub(crate) fn detect_layer(file_name: &str, source: &str) -> LayerIdentity {
    let lower = file_name.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or("");
    let contains = |patterns: &[&str]| patterns.iter().any(|pattern| lower.contains(pattern));

    if matches!(extension, "drl" | "xln" | "exc" | "ncd")
        || contains(&["-pth", "_pth", "-npth", "_npth", "drill"])
        || (extension == "txt" && looks_like_excellon(source))
    {
        return identity("drill", "Drill", "#67a6ff", true);
    }
    if extension == "gtl"
        || contains(&[
            "f_cu",
            "f.cu",
            "top_copper",
            "top-copper",
            ".front.gbr",
            "-front.gbr",
        ])
    {
        return identity("top-copper", "Top copper", "#e7a33e", false);
    }
    if extension == "gbl"
        || contains(&[
            "b_cu",
            "b.cu",
            "bottom_copper",
            "bottom-copper",
            ".back.gbr",
            "-back.gbr",
        ])
    {
        return identity("bottom-copper", "Bottom copper", "#cf7835", false);
    }
    if matches!(extension, "g1" | "g2" | "g3" | "g4") || contains(&["in1_cu", "in2_cu", "inner"]) {
        return identity("inner-copper", "Inner copper", "#c89d45", false);
    }
    if extension == "gts" || contains(&["f_mask", "f.mask", "top_mask", "top-mask"]) {
        return identity("top-soldermask", "Top soldermask", "#48a875", false);
    }
    if extension == "gbs" || contains(&["b_mask", "b.mask", "bottom_mask", "bottom-mask"]) {
        return identity("bottom-soldermask", "Bottom soldermask", "#2f7d5c", false);
    }
    if extension == "gto"
        || contains(&["f_silkscreen", "f_silk", "f.silk", "top_silk", "frontsilk"])
    {
        return identity("top-silkscreen", "Top silkscreen", "#f1f1ed", false);
    }
    if extension == "gbo"
        || contains(&[
            "b_silkscreen",
            "b_silk",
            "b.silk",
            "bottom_silk",
            "backsilk",
        ])
    {
        return identity("bottom-silkscreen", "Bottom silkscreen", "#c9c9c5", false);
    }
    if extension == "gtp" || contains(&["f_paste", "f.paste", "top_paste"]) {
        return identity("top-paste", "Top paste", "#aeb4bc", false);
    }
    if extension == "gbp" || contains(&["b_paste", "b.paste", "bottom_paste"]) {
        return identity("bottom-paste", "Bottom paste", "#8d949d", false);
    }
    if matches!(extension, "gko" | "gm1" | "gml")
        || contains(&["edge_cuts", "edge.cuts", "outline", "board_edge"])
    {
        return identity("outline", "Board outline", "#f5f5f7", false);
    }

    identity("gerber", "Gerber", "#9d8cff", false)
}

fn looks_like_excellon(source: &str) -> bool {
    let upper = source.to_ascii_uppercase();
    upper.contains("M48")
        || upper.lines().any(|line| {
            let line = line.trim();
            line.starts_with('T') && line.contains('C')
        })
}

fn identity(
    kind: &'static str,
    label: &'static str,
    color: &'static str,
    is_drill: bool,
) -> LayerIdentity {
    LayerIdentity {
        kind,
        label,
        color,
        is_drill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_standard_fabrication_extensions() {
        assert_eq!(detect_layer("board.GTL", "").kind, "top-copper");
        assert_eq!(detect_layer("board.GBL", "").kind, "bottom-copper");
        assert_eq!(detect_layer("board.GTS", "").kind, "top-soldermask");
        assert_eq!(detect_layer("board.GTO", "").kind, "top-silkscreen");
        assert_eq!(detect_layer("board.GKO", "").kind, "outline");
        assert_eq!(detect_layer("board.DRL", "").kind, "drill");
    }

    #[test]
    fn detects_kicad_names_with_generic_gerber_extensions() {
        assert_eq!(detect_layer("board-F_Cu.gbr", "").kind, "top-copper");
        assert_eq!(
            detect_layer("board-B_Mask.gbr", "").kind,
            "bottom-soldermask"
        );
        assert_eq!(detect_layer("board-Edge_Cuts.gbr", "").kind, "outline");
    }
}
