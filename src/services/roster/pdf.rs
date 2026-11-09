//! A4 tables with embedded Unicode font, repeated headers, and wrapped cells.
use super::*;
use std::collections::{BTreeSet, HashMap};
const FONT: &[u8] = include_bytes!("assets/NotoSans-Regular.ttf");
struct Font<'a> {
    face: ttf_parser::Face<'a>,
    cids: HashMap<char, u16>,
}
impl Font<'_> {
    fn width(&self, text: &str, size: f64) -> f64 {
        text.chars()
            .map(|c| {
                self.face
                    .glyph_index(c)
                    .and_then(|g| self.face.glyph_hor_advance(g))
                    .unwrap_or(600) as f64
                    / self.face.units_per_em() as f64
                    * size
            })
            .sum()
    }
    fn hex(&self, text: &str) -> String {
        text.chars()
            .map(|c| format!("{:04X}", self.cids.get(&c).copied().unwrap_or(0)))
            .collect()
    }
    fn wrap(&self, text: &str, width: f64, size: f64) -> Vec<String> {
        let mut lines = vec![];
        let mut line = String::new();
        for ch in text.chars() {
            if ch == '\n' {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            let ch = if ch.is_control() { ' ' } else { ch };
            let mut next = line.clone();
            next.push(ch);
            if self.width(&next, size) > width && !line.is_empty() {
                if let Some(i) = line.rfind(' ') {
                    let rest = line.split_off(i + 1);
                    lines.push(line.trim_end().to_owned());
                    line = rest;
                } else {
                    lines.push(std::mem::take(&mut line));
                }
            }
            line.push(ch);
        }
        lines.push(line);
        lines
    }
    fn text(&self, content: &mut String, text: &str, x: f64, y: f64, size: f64) {
        content.push_str(&format!(
            "BT /F1 {size:.2} Tf 1 0 0 1 {x:.2} {y:.2} Tm <{}> Tj ET\n",
            self.hex(text)
        ));
    }
}
fn stream(bytes: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", bytes.len()).into_bytes();
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\nendstream");
    out
}
fn table_row(
    font: &Font<'_>,
    content: &mut String,
    cells: &[Vec<String>],
    top: f64,
    width: f64,
    height: f64,
    header: bool,
) {
    for (j, lines) in cells.iter().enumerate() {
        let x = 24.0 + j as f64 * width;
        if header {
            content.push_str(&format!(
                "0.92 g {x:.2} {:.2} {width:.2} {height:.2} re f 0 g\n",
                top - height
            ));
        }
        content.push_str(&format!(
            "0.7 G 0.4 w {x:.2} {:.2} {width:.2} {height:.2} re S 0 G\n",
            top - height
        ));
        for (i, line) in lines.iter().enumerate() {
            font.text(content, line, x + 4.0, top - 12.0 - i as f64 * 10.0, 7.4);
        }
    }
}
fn page_start(
    font: &Font<'_>,
    title: &str,
    page: usize,
    header: &[Vec<String>],
    width: f64,
) -> (String, f64) {
    let mut content = String::new();
    let lines = font.wrap(title, 500.0, 10.0);
    let mut y = 812.0;
    for line in lines {
        font.text(&mut content, &line, 24.0, y, 10.0);
        y -= 14.0;
    }
    font.text(&mut content, &format!("Page {page}"), 535.0, 812.0, 7.0);
    y -= 8.0;
    let height = header.iter().map(Vec::len).max().unwrap_or(1) as f64 * 10.0 + 8.0;
    table_row(font, &mut content, header, y, width, height, true);
    (content, y - height)
}
pub fn render(rows: &[Vec<String>], title: &str) -> Result<Vec<u8>> {
    let face = ttf_parser::Face::parse(FONT, 0)
        .map_err(|_| ApiError::new(500, "EXPORT_FAILED", "PDF font unavailable"))?;
    let mut chars: BTreeSet<char> = (32u8..=126).map(char::from).collect();
    for text in rows
        .iter()
        .flatten()
        .map(String::as_str)
        .chain(std::iter::once(title))
    {
        chars.extend(text.chars().filter(|c| !c.is_control()));
    }
    if chars.len() > 65534 {
        return Err(ApiError::bad(
            "Too many distinct characters for a PDF export",
        ));
    }
    let cids: HashMap<_, _> = chars
        .iter()
        .enumerate()
        .map(|(i, c)| (*c, (i + 1) as u16))
        .collect();
    let font = Font { face, cids };
    let scale = 1000.0 / font.face.units_per_em() as f64;
    let mut widths = vec![];
    let mut glyphs = vec![0u8; 2 * (chars.len() + 1)];
    let mut mappings = vec![];
    for (c, cid) in &font.cids {
        let gid = font.face.glyph_index(*c).unwrap_or(ttf_parser::GlyphId(0));
        glyphs[*cid as usize * 2..*cid as usize * 2 + 2].copy_from_slice(&gid.0.to_be_bytes());
        let w = (font.face.glyph_hor_advance(gid).unwrap_or(600) as f64 * scale).round() as i64;
        widths.push((*cid, w));
        let unicode = c
            .encode_utf16(&mut [0; 2])
            .iter()
            .map(|u| format!("{u:04X}"))
            .collect::<String>();
        mappings.push(format!("<{cid:04X}> <{unicode}>"));
    }
    widths.sort_by_key(|(cid, _)| *cid);
    let mut cmap="/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /RosterUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n".to_owned();
    for chunk in mappings.chunks(100) {
        cmap.push_str(&format!(
            "{} beginbfchar\n{}\nendbfchar\n",
            chunk.len(),
            chunk.join("\n")
        ));
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    let bounds = font.face.global_bounding_box();
    let bbox = [bounds.x_min, bounds.y_min, bounds.x_max, bounds.y_max]
        .map(|n| (n as f64 * scale).round() as i64);
    let mut objects:Vec<Vec<u8>>=vec![vec![],vec![],b"<< /Type /Font /Subtype /Type0 /BaseFont /NotoSans /Encoding /Identity-H /DescendantFonts [4 0 R] /ToUnicode 7 0 R >>".to_vec(),format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /NotoSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 5 0 R /CIDToGIDMap 8 0 R /DW 600 /W [1 [{}]] >>",widths.iter().map(|(_,w)|w.to_string()).collect::<Vec<_>>().join(" ")).into_bytes(),format!("<< /Type /FontDescriptor /FontName /NotoSans /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight 714 /StemV 80 /FontFile2 6 0 R >>",bbox[0],bbox[1],bbox[2],bbox[3],(font.face.ascender()as f64*scale).round(),(font.face.descender()as f64*scale).round()).into_bytes(),stream(FONT,&format!("/Length1 {}",FONT.len())),stream(cmap.as_bytes(),""),stream(&glyphs,"")];
    let columns = rows.first().map(Vec::len).unwrap_or(1).max(1);
    let width = 547.0 / columns as f64;
    let header: Vec<_> = rows
        .first()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|t| font.wrap(t, width - 8.0, 7.4))
        .collect();
    let (mut content, mut y) = page_start(&font, title, 1, &header, width);
    let mut pages = vec![];
    let page_capacity = ((y - 32.0) / 10.0).floor().max(0.0) as usize;
    if page_capacity == 0 {
        return Err(ApiError::bad("PDF title exceeds printable page height"));
    }
    for row in rows.iter().skip(1) {
        let cells: Vec<_> = row.iter().map(|t| font.wrap(t, width - 8.0, 7.4)).collect();
        let max_lines = cells.iter().map(Vec::len).max().unwrap_or(1);
        let mut offset = 0;
        while offset < max_lines {
            let available = ((y - 24.0 - 8.0) / 10.0).floor().max(0.0) as usize;
            if available == 0
                || (offset == 0 && max_lines <= page_capacity && max_lines > available)
            {
                pages.push(content);
                (content, y) = page_start(&font, title, pages.len() + 1, &header, width);
                continue;
            }
            let count = (max_lines - offset).min(available);
            let part: Vec<Vec<String>> = cells
                .iter()
                .map(|lines| lines.iter().skip(offset).take(count).cloned().collect())
                .collect();
            let height = count as f64 * 10.0 + 8.0;
            table_row(&font, &mut content, &part, y, width, height, false);
            y -= height;
            offset += count;
        }
    }
    pages.push(content);
    let mut kids = vec![];
    for content in pages {
        let page_id = objects.len() + 1;
        let content_id = page_id + 1;
        kids.push(format!("{page_id} 0 R"));
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>").into_bytes());
        objects.push(stream(content.as_bytes(), ""));
    }
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        kids.len()
    )
    .into_bytes();
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![];
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
            objects.len() + 1
        )
        .as_bytes(),
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn long_unicode_rosters_repeat_headers_across_pages() {
        let mut rows = vec![vec!["Member".into(), "Phone".into(), "Reason".into()]];
        for i in 0..150 {
            rows.push(vec![
                format!("Ṣọlá Adéyẹmí {i}"),
                "08031234567".into(),
                "Travelling with the extended family and returning after the service.".repeat(3),
            ]);
        }
        let bytes = super::render(
            &rows,
            "The General Assembly 2026 | Day 1 Morning | Report 07:30 | Start 08:45",
        )
        .unwrap();
        let pdf = String::from_utf8_lossy(&bytes);
        assert!(pdf.matches("/Type /Page /Parent").count() > 2);
        assert!(pdf.contains("/ToUnicode 7 0 R"));
        if let Ok(path) = std::env::var("TGA_PDF_QA") {
            std::fs::write(path, bytes).unwrap();
        }
    }
}
