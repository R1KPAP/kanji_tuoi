use anyhow::{Context, Result, anyhow};
use encoding_rs::EUC_JP;
use kanji_tui::model::{KanjiData, KanjiStrokes, MyPoint};
use lyon_geom::cubic_bezier::CubicBezierSegment;
use lyon_geom::point;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use rayon::prelude::*;
use serde::Deserialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::Path,
};
use usvg::tiny_skia_path::PathSegment;
use usvg::{Group, Node, Options, Path as UsvgPath, Tree};
use wana_kana::ConvertJapanese;

enum NextXMLTag<'a> {
    ToSkip,
    Kanji,
    RadIndex,
    Grade,
    Strokes,
    Frequece,
    Jlpt,
    Reading(&'a str),
    Meaning,
    Nanori,
    RadName,
}
#[derive(Deserialize)]
struct KanjiInfo {
    name: String,
    variants: Option<Vec<char>>,
}

fn main() {
    let kanji_info = load_radicals_dataset("assets/radicals.json").unwrap();
    let (mut full_dataset, kanji_to_freq) =
        parse_kanjidic2_xml_data("assets/full_kanji_dic2.xml", &kanji_info).unwrap();
    let krad_data = parse_krad_data("assets/kradfile").unwrap();
    let radk_data = build_reverse_krad_map(&krad_data);
    enrich_dataset(
        &mut full_dataset,
        &radk_data,
        &krad_data,
        &kanji_to_freq,
        &kanji_info,
    );
    //save_dataset_as_json(&full_dataset, "data/full_dataset.json").unwrap();
    save_dataset_as_binary(&full_dataset, "data/kanji_dataset.bin").unwrap();
}

/// Parse KANJIDIC2 xml, extract data into JSON
fn parse_kanjidic2_xml_data(
    path: &str,
    radicals: &HashMap<char, KanjiInfo>,
) -> Result<(Vec<KanjiData>, HashMap<char, u16>)> {
    let mut result = Vec::<KanjiData>::new();
    let mut kanji_to_freq = HashMap::new();

    let xml_content = fs::read_to_string(path).context("Error opening KANJIDIC2 xml")?;
    let mut xml_reader = Reader::from_str(&xml_content);
    xml_reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut next_xml_tag = NextXMLTag::ToSkip;
    let mut current_kanji: Option<KanjiData> = None;

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                next_xml_tag = match e.name().as_ref() {
                    "character" => {
                        current_kanji = Some(KanjiData::default());
                        NextXMLTag::ToSkip
                    }
                    "rad_name" => NextXMLTag::RadName,
                    "literal" => NextXMLTag::Kanji,
                    "grade" => NextXMLTag::Grade,
                    "freq" => NextXMLTag::Frequece,
                    "stroke_count" => NextXMLTag::Strokes,
                    "jlpt" => NextXMLTag::Jlpt,
                    "meaning" if e.attributes().next().is_none() => NextXMLTag::Meaning,
                    "reading" => e
                        .attributes()
                        .filter_map(Result::ok)
                        .find(|attr| attr.key.as_ref() == "r_type")
                        .map_or(NextXMLTag::ToSkip, |attr| match attr.value.as_ref() {
                            "ja_on" => NextXMLTag::Reading("ON"),
                            "ja_kun" => NextXMLTag::Reading("KUN"),
                            _ => NextXMLTag::ToSkip,
                        }),
                    "nanori" => NextXMLTag::Nanori,
                    "rad_value" => NextXMLTag::RadIndex,
                    _ => NextXMLTag::ToSkip,
                };
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut kanji) = current_kanji {
                    if let Ok(text_str) = std::str::from_utf8(e.as_bytes()) {
                        let text_str = text_str.trim();
                        if !text_str.is_empty() {
                            match next_xml_tag {
                                NextXMLTag::Kanji => {
                                    kanji.kanji = text_str.chars().next().unwrap_or(' ')
                                }
                                NextXMLTag::RadIndex => {
                                    kanji.rad_index = text_str.parse::<u8>().ok()
                                }
                                NextXMLTag::Grade => kanji.grade = text_str.parse::<u8>().ok(),
                                NextXMLTag::Strokes => {
                                    kanji.strokes_count = text_str.parse::<u8>().ok()
                                }
                                NextXMLTag::Frequece => {
                                    kanji.freq_rank = text_str.parse::<u16>().ok()
                                }
                                NextXMLTag::Jlpt => kanji.jlpt_level = text_str.parse::<u8>().ok(),
                                NextXMLTag::RadName => kanji.names.push(text_str.to_string()),
                                NextXMLTag::Reading("ON") => {
                                    kanji.onyomi.push(text_str.to_string());
                                    kanji.romaji.push(text_str.to_string().to_romaji());
                                }
                                NextXMLTag::Reading("KUN") => {
                                    kanji.kunyomi.push(text_str.to_string());
                                    kanji.romaji.push(text_str.to_string().to_romaji());
                                }
                                NextXMLTag::Meaning => kanji.meanings.push(text_str.to_string()),
                                NextXMLTag::Nanori => kanji.nanori.push(text_str.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == "character" {
                    if let Some(mut kanji) = current_kanji.take() {
                        let radical_info = radicals.get(&kanji.kanji);

                        let variants = radical_info
                            .and_then(|info| info.variants.as_deref())
                            .unwrap_or(&[]);
                        let rad_name = radical_info
                            .map(|info| info.name.clone())
                            .unwrap_or("".to_string());

                        let mut kanji_names = Vec::with_capacity(1 + variants.len());
                        kanji_names.push(kanji.kanji);
                        kanji_names.extend_from_slice(&variants);

                        kanji_to_freq.insert(kanji.kanji, kanji.freq_rank.unwrap_or(u16::MAX));
                        kanji.variants = variants.to_vec();
                        kanji.names.push(rad_name);
                        result.push(kanji);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("Error while XML KANJIDIC2 parsing: {e}"),
            _ => (),
        }
        buf.clear();
    }

    Ok((result, kanji_to_freq))
}

/// Parse krad file
fn parse_krad_data(path: &str) -> Result<HashMap<char, Vec<char>>> {
    let bytes = fs::read(path).context("Error opening krad file")?;
    let (cow_str, _encoding_used, had_errors) = EUC_JP.decode(&bytes);
    if had_errors {
        anyhow::bail!("Error while krad file EUC-JP decode");
    }
    let mut krad_map = HashMap::new();

    for line in cow_str.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((kanji, radicals)) = line.split_once(':') {
            let kanji_key: char = kanji.chars().next().unwrap_or_default();
            let radical_list: Vec<char> = radicals
                .split_whitespace()
                .filter_map(|s| s.chars().next())
                .collect();

            krad_map.insert(kanji_key, radical_list);
        }
    }
    Ok(krad_map)
}

/// Parse radk file
pub fn parse_radk_data(path: &str) -> Result<HashMap<char, Vec<char>>> {
    let bytes = fs::read(path).context("Error opening radk file")?;

    let (cow_str, _encoding, had_errors) = EUC_JP.decode(&bytes);
    if had_errors {
        anyhow::bail!("Error while radk file EUC-JP decode");
    }

    let mut radk_map: HashMap<char, Vec<char>> = HashMap::new();
    let mut current_radical: Option<char> = None;

    for line in cow_str.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('$') {
            if let Some(rad_char) = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.chars().next())
            {
                current_radical = Some(rad_char);
            }
            continue;
        }

        if let Some(ref rad) = current_radical {
            let kanji_on_this_line = line.chars().filter(|c| !c.is_whitespace());

            radk_map.entry(*rad).or_default().extend(kanji_on_this_line);
        }
    }
    Ok(radk_map)
}

fn enrich_dataset(
    data_set: &mut Vec<KanjiData>,
    radk_data: &HashMap<char, Vec<char>>,
    krad_data: &HashMap<char, Vec<char>>,
    kanji_to_freq: &HashMap<char, u16>,
    kanji_info: &HashMap<char, KanjiInfo>,
) {
    let dataset_kanji: HashSet<char> = data_set
        .iter()
        .flat_map(|k| {
            let mut variants = k.variants.clone();
            variants.push(k.kanji);
            variants
        })
        .collect();

    let not_found: Vec<char> = kanji_info
        .keys()
        .filter(|&k| !dataset_kanji.contains(k))
        .copied()
        .collect();

    let decomposition = not_found
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    println!("notFound: {}", decomposition);

    let info_matches = AtomicUsize::new(0);
    data_set.par_iter_mut().for_each(|kanji_data| {
        if let Some(kanji_info) = kanji_info.get(&kanji_data.kanji) {
            info_matches.fetch_add(1, Ordering::Relaxed);
            kanji_data.names = vec![kanji_info.name.clone()];
            kanji_data.variants = kanji_info.variants.clone().unwrap_or_default();
        }
        kanji_data.decomposition = krad_data
            .get(&kanji_data.kanji)
            .cloned()
            .unwrap_or_default();

        kanji_data.used_in = radk_data
            .get(&kanji_data.kanji)
            .map(|list| {
                let mut sorted = list.clone();
                sorted.sort_unstable_by_key(|c| kanji_to_freq.get(c).copied().unwrap_or(u16::MAX));
                sorted.truncate(10);
                sorted
            })
            .unwrap_or_default();

        if let Ok(kanji_strokes) = extract_kanji_svg("assets/kanji_svg", &kanji_data.kanji, 0.5) {
            kanji_data.strokes = kanji_strokes;
        }
    });
    println!(
        "Elementi trovati in kanji_info: {}",
        info_matches.load(Ordering::Relaxed)
    );
}

pub fn extract_kanji_svg(
    svg_dir_path: &str,
    kanji_name: &char,
    tolerance: f32,
) -> Result<Vec<KanjiStrokes>> {
    let opt = Options::default();
    let svg_path = Path::new(svg_dir_path)
        .join(format!("{:05x}.svg", *kanji_name as u32));

    let svg_content = fs::read_to_string(&svg_path)
        .with_context(|| format!("Impossibile to open the SVG file: {:?}", svg_path))?;

    let cleaned_svg = svg_content.replace("kvg:", "kvg_");

    let tree = Tree::from_str(&cleaned_svg, &opt).context("Error while parsing the SVG file")?;

    let mut result: Vec<KanjiStrokes> = Vec::new();

    let mut paths = Vec::new();
    collect_paths(tree.root(), &mut paths);

    for path in paths {
        let mut points = Vec::new();
        let mut current_pos = point(0.0, 0.0);

        for command in path.data().segments() {
            match command {
                PathSegment::MoveTo(p) | PathSegment::LineTo(p) => {
                    current_pos = point(p.x, p.y);
                    points.push(MyPoint { x: p.x, y: p.y });
                }
                PathSegment::CubicTo(p1, p2, p3) => {
                    let bezier = CubicBezierSegment {
                        from: current_pos,
                        ctrl1: point(p1.x, p1.y),
                        ctrl2: point(p2.x, p2.y),
                        to: point(p3.x, p3.y),
                    };

                    points.extend(
                        bezier
                            .flattened(tolerance)
                            .skip(1)
                            .map(|p| MyPoint::from_xy(p.x, p.y)),
                    );
                    current_pos = point(p3.x, p3.y);
                }
                PathSegment::Close => {}
                _ => {}
            }
        }

        if !points.is_empty() {
            let order = result.len() as u8;
            result.push(KanjiStrokes {
                strokes: points,
                order,
            });
        }
    }
    Ok(result)
}

fn load_radicals_dataset(path: &str) -> Result<HashMap<char, KanjiInfo>> {
    let file =
        File::open(path).with_context(|| format!("Error opening radicals file: {}", path))?;
    let reader = BufReader::new(file);
    let dataset: HashMap<char, KanjiInfo> = serde_json::from_reader(reader)
        .with_context(|| format!("Error deserializing radicals JSON: {}", path))?;
    Ok(dataset)
}

fn collect_paths<'a>(group: &'a Group, paths: &mut Vec<&'a UsvgPath>) {
    for node in group.children() {
        match node {
            Node::Path(path) => paths.push(path),
            Node::Group(sub_group) => collect_paths(sub_group, paths),
            _ => {}
        }
    }
}

fn save_dataset_as_binary(dataset: &[KanjiData], path: &str) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("Impossibile to create the binary file: {}", path))?;
    let writer = BufWriter::new(file);

    bincode::serialize_into(writer, dataset).context("Error while writing bytes on the file")?;

    println!("Dataset correctly saved at: {}", path);
    Ok(())
}

fn build_reverse_krad_map(krad_data: &HashMap<char, Vec<char>>) -> HashMap<char, Vec<char>> {
    let mut reverse_map: HashMap<char, Vec<char>> = HashMap::new();
    for (&kanji, components) in krad_data {
        for &comp in components {
            reverse_map.entry(comp).or_default().push(kanji);
        }
    }
    reverse_map
}

fn save_dataset_as_json(dataset: &[KanjiData], path: &str) -> Result<()> {
    println!("Saving the full data set...");
    let out_file = std::fs::File::create(path).context("Impossible to create the output file")?;
    let writer = BufWriter::new(out_file);

    serde_json::to_writer_pretty(writer, &dataset)
        .context("Error while serializing data set to JSON")?;

    println!("JSON data set saved successfully");
    Ok(())
}
