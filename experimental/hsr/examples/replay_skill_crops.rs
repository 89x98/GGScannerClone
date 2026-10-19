//! Offline diagnostic: reuse the production OCR/parser on saved trace frames.
//! Usage: replay_skill_crops <dump-root> <capture.json> <run-dir> <report.json>
use std::{collections::BTreeMap, env, fs, path::PathBuf};

use hsr_scanner::{
    annotator, load_embedded_gilore_reference,
    ocr::{PaddleOcrReader, PanelParser},
};
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).map(PathBuf::from).collect();
    if !(4..=5).contains(&args.len()) {
        return Err(
            "usage: replay_skill_crops <dump-root> <capture.json> <run-dir> <report.json> [runtime-reference-cache.json]".into(),
        );
    }
    let root = args[0].canonicalize()?;
    let truth: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let output = if args[3].is_absolute() {
        args[3].clone()
    } else {
        env::current_dir()?.join(&args[3])
    };
    let refs = if let Some(path) = args.get(4) {
        let disk: Value = serde_json::from_slice(&fs::read(path)?)?;
        let document: hsr_scanner::data_cache::SharedDataDocument =
            serde_json::from_value(disk["data"].clone())?;
        document.validate()?
    } else {
        load_embedded_gilore_reference()?
    };
    fs::create_dir_all(&args[2])?;
    env::set_current_dir(args[2].canonicalize()?)?;
    annotator::init(true);
    let mut parser = PanelParser::new(PaddleOcrReader::new()?);
    let mut manifests = fs::read_dir(root.join("hsr_character_identity"))?
        .map(|e| e.map(|x| x.path()))
        .collect::<Result<Vec<_>, _>>()?;
    manifests.sort();
    let mut records = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for folder in manifests {
        let identity: Value = serde_json::from_slice(&fs::read(folder.join("ocr_fields.json"))?)?;
        let Some(id) = identity["fields"]
            .as_array()
            .and_then(|fields| fields.iter().find(|e| e["field"] == "character_name"))
            .and_then(|e| e["final"].as_str())
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        let index: usize = folder.file_name().unwrap().to_string_lossy().parse()?;
        let eidolon: Value = serde_json::from_slice(&fs::read(
            root.join("hsr_character_eidolons")
                .join(format!("{index:04}"))
                .join("ocr_fields.json"),
        )?)?;
        let rank: u8 = eidolon["final_object"]
            .as_str()
            .and_then(|s| s.strip_prefix("eidolon="))
            .ok_or("missing observed eidolon")?
            .parse()?;
        let trace_dir = root
            .join("hsr_character_traces")
            .join(format!("{index:04}"));
        let frame = image::open(trace_dir.join("full.png"))?.to_rgb8();
        let reference = refs
            .character(id)
            .ok_or_else(|| format!("unknown character reference {id}"))?;
        let expected = truth["characters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"].as_str() == Some(id.to_string().as_str()))
            .ok_or("character absent from capture")?;
        annotator::begin_item("character_traces", index, &frame);
        annotator::add_image("full", &frame);
        match parser.parse_character_traces(&frame, &reference.path, rank) {
            Ok(details) => {
                annotator::finalize_success(&serde_json::to_string(&details)?);
                let mut checks = Vec::new();
                for key in ["basic", "skill", "ult", "talent"]
                    .into_iter()
                    .chain((reference.path == "Elation").then_some("elation"))
                {
                    let got = details.skills.get(key).copied();
                    checks.push(json!({"field":format!("skills.{key}"),"expected":expected["skills"][key],"observed":got,"matches":expected["skills"][key].as_u64()==got.map(u64::from)}));
                }
                let memo: BTreeMap<String, u32> = serde_json::from_value(
                    expected.get("memosprite").cloned().unwrap_or(json!({})),
                )?;
                for (key, value) in memo {
                    let got = details
                        .memosprite
                        .as_ref()
                        .and_then(|m| m.get(&key))
                        .copied();
                    checks.push(json!({"field":format!("memosprite.{key}"),"expected":value,"observed":got,"matches":got==Some(value)}));
                }
                records.push(json!({"id":id,"index":index,"path":reference.path,"observed_eidolon":rank,"checks":checks,"traces_omitted":details.traces.is_empty()}));
            },
            Err(error) => {
                annotator::finalize_error(None, &error.to_string());
                records.push(json!({"id":id,"index":index,"error":error.to_string()}));
            },
        }
    }
    annotator::flush();
    let mismatches = records.iter().filter(|r| r.get("error").is_some()).count()
        + records
            .iter()
            .flat_map(|r| r["checks"].as_array().into_iter().flatten())
            .filter(|c| c["matches"] != true)
            .count();
    let checks = records
        .iter()
        .flat_map(|r| r["checks"].as_array().into_iter().flatten())
        .count();
    let report = json!({"mode":"offline production Paddle OCR replay; not a new live scan","source_dump":root,"character_count":records.len(),"skill_checks":checks,"mismatches":mismatches,"records":records});
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "characters={}, skill checks={checks}, mismatches={mismatches}; report={}",
        seen.len(),
        output.display()
    );
    if mismatches != 0 {
        return Err("offline skill mismatch; inspect report and crops".into());
    }
    Ok(())
}
