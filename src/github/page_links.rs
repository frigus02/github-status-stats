pub struct PageLinks<'a> {
    pub first: Option<&'a str>,
    pub last: Option<&'a str>,
    pub next: Option<&'a str>,
    pub prev: Option<&'a str>,
}

pub fn parse(link_header: &str) -> PageLinks<'_> {
    let mut links = PageLinks {
        first: None,
        last: None,
        next: None,
        prev: None,
    };

    for link in link_header.split(',') {
        let mut segments = link.split(';');
        let url = segments.next().and_then(|url_segment| {
            if url_segment.starts_with('<') && url_segment.ends_with('>') {
                Some(&url_segment[1..url_segment.len() - 1])
            } else {
                None
            }
        });
        let rel = segments
            .find(|segment| segment.trim().starts_with("rel="))
            .map(|rel| rel.trim())
            .map(|rel| &rel[4..rel.len()]);
        if let Some(rel) = rel {
            match rel.trim_matches('"') {
                "first" => links.first = url,
                "last" => links.last = url,
                "next" => links.next = url,
                "prev" => links.prev = url,
                _ => {}
            };
        }
    }

    links
}
