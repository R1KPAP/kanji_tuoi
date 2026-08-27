use std::collections::HashSet;
use kanji_tui::model::KanjiData;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum JlptLevel {
    N(u8),
}

pub const KANGXI_RADICALS: [char; 214] = [
    '一', '丨', '丶', '丿', '乙', '亅', '二', '亠', '人', '儿', '入', '八', '冂', '冖', '冫', '几',
    '凵', '刀', '力', '勹', '匕', '匚', '匸', '十', '卜', '卩', '厂', '厶', '又', '口', '囗', '土',
    '士', '夂', '夊', '夕', '大', '女', '子', '宀', '寸', '小', '尢', '尸', '屮', '山', '川', '工',
    '己', '巾', '干', '幺', '广', '廴', '廾', '弋', '弓', '彐', '彡', '彳', '心', '戈', '戸', '手',
    '支', '攴', '文', '斗', '斤', '方', '无', '日', '曰', '月', '木', '欠', '止', '歹', '殳', '毋',
    '比', '毛', '氏', '气', '水', '火', '爪', '父', '爻', '爿', '片', '牙', '牛', '犬', '玄', '玉',
    '瓜', '瓦', '甘', '生', '用', '田', '疋', '疒', '癶', '白', '皮', '皿', '目', '矛', '矢', '石',
    '示', '禸', '禾', '穴', '立', '竹', '米', '糸', '缶', '网', '羊', '羽', '老', '而', '耒', '耳',
    '聿', '肉', '臣', '自', '至', '臼', '舌', '舛', '舟', '艮', '色', '艸', '虍', '虫', '血', '行',
    '衣', '襾', '見', '角', '言', '谷', '豆', '豕', '豸', '貝', '赤', '走', '足', '身', '車', '辛',
    '辰', '辵', '邑', '酉', '釆', '里', '金', '長', '門', '阜', '隷', '隹', '雨', '靑', '非', '面',
    '革', '韋', '韭', '音', '頁', '風', '飛', '食', '首', '香', '馬', '骨', '高', '髟', '鬥', '鬯',
    '鬲', '鬼', '魚', '鳥', '鹵', '鹿', '麥', '麻', '黄', '黍', '黒', '黹', '黽', '鼎', '鼓', '鼠',
    '鼻', '齊', '齒', '龍', '龜', '龠',
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KanjiGrade {
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    Joyo,
    Jinmeiyo,
    Hyogai,
}

impl KanjiGrade {
    pub fn from_num(num: u8) -> Self {
        match num {
            1 => KanjiGrade::G1,
            2 => KanjiGrade::G2,
            3 => KanjiGrade::G3,
            4 => KanjiGrade::G4,
            5 => KanjiGrade::G5,
            6 => KanjiGrade::G6,
            7 => KanjiGrade::Joyo,
            8 => KanjiGrade::Jinmeiyo,
            _ => KanjiGrade::Hyogai,
        }
    }

    fn to_num(&self) -> u8 {
        match self {
            KanjiGrade::G1 => 1,
            KanjiGrade::G2 => 2,
            KanjiGrade::G3 => 3,
            KanjiGrade::G4 => 4,
            KanjiGrade::G5 => 5,
            KanjiGrade::G6 => 6,
            KanjiGrade::Joyo => 8,
            KanjiGrade::Jinmeiyo => 9,
            KanjiGrade::Hyogai => 11,
        }
    }

    pub fn descs() -> &'static [(KanjiGrade, &'static str, usize)] {
        &[
            (KanjiGrade::G1, "G1", 0),
            (KanjiGrade::G2, "G2", 1),
            (KanjiGrade::G3, "G3", 2),
            (KanjiGrade::G4, "G4", 3),
            (KanjiGrade::G5, "G5", 4),
            (KanjiGrade::G6, "G6", 5),
            (KanjiGrade::Joyo, "Jōyō", 6),
            (KanjiGrade::Jinmeiyo, "Jinmeiyō", 7),
            (KanjiGrade::Hyogai, "Hyōgai", 8),
        ]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RangeFilter<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T: PartialOrd> RangeFilter<T> {
    pub fn new(min: Option<T>, max: Option<T>) -> Self {
        Self { min, max }
    }

    pub fn matches(&self, value: T) -> bool {
        self.min.as_ref().is_none_or(|min| &value >= min)
            && self.max.as_ref().is_none_or(|max| &value <= max)
    }
}

#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub jlpt: Option<HashSet<JlptLevel>>,
    pub grade: Option<HashSet<KanjiGrade>>,
    pub strokes: Option<RangeFilter<u8>>,
    pub rank: Option<RangeFilter<u16>>,
    pub rad_index: Option<HashSet<u8>>,

    pub query_text: Option<String>,
}

impl Default for SearchCriteria {
    fn default() -> Self {
        let jlpt_space = HashSet::from([JlptLevel::N(1)]);
        Self {
            jlpt: Some(jlpt_space),
            grade: None,
            strokes: None,
            rank: None,
            rad_index: None,
            query_text: None,
        }
    }
}

pub trait KanjiRepo {
    fn search(&self, search_criteria: &SearchCriteria) -> Vec<&KanjiData>;
    fn get_index_by_kanji(&self, kanji: char) -> Option<usize>;
    fn get_by_kanji(&self, kanji: char) -> Option<&KanjiData>;
    fn get_by_index(&self, index: usize) -> Option<&KanjiData>;
}

//Conditional compilation for local json
#[cfg(feature = "json-backend")]
pub mod json;
