pub(in crate::domain::library::service) fn parse_lyric_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| line.split(']').next_back())
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !line.contains("作词")
                && !line.contains("作曲")
                && !line.contains("纯音乐")
                && !line.contains("编曲")
        })
        .map(|line| line.to_string())
        .collect()
}
