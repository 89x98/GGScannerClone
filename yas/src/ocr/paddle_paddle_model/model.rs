use crate::ocr::paddle_paddle_model::preprocess::{normalize_image_to_ndarray, resize_img};
use crate::ocr::ImageToText;
use crate::positioning::Shape3D;
use anyhow::Result;
use image::RgbImage;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Value,
};
use std::{path::Path, sync::Mutex, time::Duration};

/// Both desktop platforms use the same ONNX Runtime inference and CTC decoder.
pub struct PPOCRModel {
    index_to_word: Vec<String>,
    model: Mutex<Session>,
}

fn parse_index_to_word(s: &str, use_whitespace: bool) -> Vec<String> {
    let mut result: Vec<String> = s.lines().map(String::from).collect();
    if use_whitespace {
        result.push(String::from(" "));
    }
    result
}

impl PPOCRModel {
    pub fn new_from_file<P1: AsRef<Path>, P2: AsRef<Path>>(
        onnx_file: P1,
        words_file: P2,
    ) -> Result<Self> {
        let words = std::fs::read_to_string(words_file)?;
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .commit_from_file(onnx_file)?;
        Ok(Self {
            index_to_word: parse_index_to_word(&words, true),
            model: Mutex::new(session),
        })
    }

    pub fn new(onnx: &[u8], index_to_word: Vec<String>) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(1)?
            .commit_from_memory(onnx)?;
        Ok(Self {
            index_to_word,
            model: Mutex::new(session),
        })
    }

    pub fn get_average_inference_time(&self) -> Option<Duration> {
        None
    }
}

impl ImageToText<RgbImage> for PPOCRModel {
    fn image_to_text(&self, image: &RgbImage, is_preprocessed: bool) -> Result<String> {
        let resized_image = if is_preprocessed {
            image.clone()
        } else {
            resize_img(Shape3D::new(3, 48, 320), image)
        };
        let tensor = normalize_image_to_ndarray(&resized_image);
        let tensor_value = Value::from_array(tensor)?;
        let mut model = self.model.lock().unwrap();
        let result = model.run(ort::inputs![tensor_value])?;
        let (shape, data) = result[0].try_extract_tensor::<f32>()?;
        let shape_dims = shape.as_ref();
        let mut text_index = Vec::new();
        for i in 0..(shape_dims[1] as usize) {
            let mut max_index = 0;
            let mut max_value = -f32::INFINITY;
            for j in 0..(shape_dims[2] as usize) {
                let value = data[i * (shape_dims[2] as usize) + j];
                if value > max_value {
                    max_value = value;
                    max_index = j;
                }
            }
            text_index.push(max_index);
        }

        // CTC 解码：与 Python 代码逻辑一致
        let mut s = String::new();
        let mut last_index: i32 = -1;
        let mut out_of_bounds_count = 0;

        for (_pos, &index) in text_index.iter().enumerate() {
            // 如果当前索引不为0且与上一个索引不同，则添加到结果
            if index != 0 && (index as i32) != last_index {
                // index 是从 1 开始的，所以访问 index_to_word[index - 1]
                // 有效范围: index ∈ [1, index_to_word.len()]
                if index == 0 || (index as usize) > self.index_to_word.len() {
                    out_of_bounds_count += 1;
                    if out_of_bounds_count == 1 {
                        // 只在第一次越界时记录详细信息
                        log_debug!("PaddleOCR 索引越界: index={}, 有效范围=[1, {}], 位置={}, 已跳过该字符",
                                   "PaddleOCR index out of bounds: index={}, valid range=[1, {}], position={}, character skipped",
                                   index, self.index_to_word.len(), _pos);
                    }
                } else {
                    let ch = &self.index_to_word[index - 1];
                    // log::info!("[OCR调试] 位置 {}: 索引 {} -> '{}'", _pos, index, ch);
                    s.push_str(ch);
                }
            }
            // 总是更新 last_index（包括0）
            last_index = index as i32;
        }

        if out_of_bounds_count > 0 {
            log_debug!(
                "PaddleOCR 本次识别共有 {} 个字符索引越界被跳过，最终识别结果: '{}'",
                "PaddleOCR {} characters skipped due to index out of bounds, final result: '{}'",
                out_of_bounds_count,
                s
            );
        }

        // log::info!("[OCR调试] 最终识别结果: {}", s);

        // println!("{:?}", text_index);

        // let s = format!("{:?}", shape);

        Ok(s)
    }

    fn get_average_inference_time(&self) -> Option<Duration> {
        None
    }
}

#[macro_export]
macro_rules! ppocr_model {
    ($onnx:literal, $index_to_word:literal) => {{
        let model_bytes = include_bytes!($onnx);
        let index_to_word_str = include_str!($index_to_word);

        let mut index_to_word_vec: Vec<String> = Vec::new();
        for line in index_to_word_str.lines() {
            index_to_word_vec.push(String::from(line));
        }
        index_to_word_vec.push(String::from(" "));

        $crate::ocr::PPOCRModel::new(model_bytes, index_to_word_vec)
    }};
}

pub struct PPOCRChV4RecInfer {
    model: PPOCRModel,
}

impl PPOCRChV4RecInfer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            model: ppocr_model!("./ch_PP-OCRv4_rec_infer.onnx", "./ppocr_keys_v1.txt")?,
        })
    }
}

impl ImageToText<RgbImage> for PPOCRChV4RecInfer {
    fn image_to_text(&self, image: &RgbImage, is_preprocessed: bool) -> Result<String> {
        self.model.image_to_text(image, is_preprocessed)
    }

    fn get_average_inference_time(&self) -> Option<Duration> {
        self.model.get_average_inference_time()
    }
}
