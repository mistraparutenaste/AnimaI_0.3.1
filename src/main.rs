use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use rust_embed::{Embed, RustEmbed};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    net::SocketAddr,
    path::Path,
    sync::{Mutex, OnceLock},
    time::Instant,
};

// ==========================================================================
// Embedded Web Assets
// ==========================================================================
#[derive(RustEmbed)]
#[folder = "."]
#[include = "index.html"]
#[include = "style.css"]
#[include = "app.js"]
#[include = "icon.png"]
#[include = "logo.png"]
struct Asset;

// ==========================================================================
// Data Structures
// ==========================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromptItem {
    japanese: String,
    prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FavoritePreset {
    name: String,
    positive: Vec<String>,
    negative: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct CategoryGroup {
    name: String,
    filename: String,
    tags: Vec<PromptItem>,
}

#[derive(Debug, Serialize, Clone)]
struct PromptsResponse {
    sfw: Vec<CategoryGroup>,
    nsfw: Vec<CategoryGroup>,
    negative: Vec<CategoryGroup>,
}

// ==========================================================================
// Hardcoded Default CSV Contents (For first-run bootstrapping)
// ==========================================================================
const DEFAULT_CSVS: &[(&str, &str)] = &[
    ("quality.csv", "Japanese,Prompt\n最高画質,\"masterpiece\"\n超高画質,\"best quality\"\n高詳細,\"ultra-detailed\"\n4K解像度,\"4k resolution\"\n8K解像度,\"8k resolution\"\n超高解像度,\"extremely detailed\"\n美麗イラスト,\"beautiful detailed\"\n高解像度,\"highres\"\n傑作,\"masterpiece, best quality\"\n極限の詳細,\"depth of field, cinematic lighting, masterpiece, best quality, ultra-detailed\"\nイラスト向け最高画質,\"masterpiece, best quality, illustration, beautiful detailed\"\n実写向け最高画質,\"masterpiece, best quality, photorealistic, ultra-detailed, 8k resolution\""),
    ("art_style.csv", "Japanese,Prompt\nアニメ風,\"anime style\"\n実写風/リアル,\"photorealistic\"\n3Dレンダリング,\"3d render\"\nちびキャラ,\"chibi\"\n水彩画風,\"watercolor style\"\n油絵風,\"oil painting style\"\nスケッチ/素描,\"sketch\"\nドット絵,\"pixel art\"\nレトロゲーム風,\"retro style\"\nサイバーパンク風,\"cyberpunk style\"\nファンタジー風,\"fantasy style\"\n浮世絵風,\"ukiyoe style\"\nパステルカラー,\"pastel colors\"\nモノクロ,\"monochrome\"\n水墨画風,\"ink brush painting\"\nポップアート,\"pop art\"\nアールヌーボー,\"art nouveau\"\nスタジオジブリ風,\"studio ghibli style\"\n新海誠風,\"makoto shinkai style\"\n京都アニメーション風,\"kyoto animation style\"\nコンセプトアート,\"concept art\""),
    ("subject.csv", "Japanese,Prompt\n女の子1人,\"1girl\"\n男の子1人,\"1boy\"\n女の子2人,\"2girls\"\n男の子2人,\"2boys\"\n男女カップル,\"1girl 1boy, couple\"\n女性,\"woman\"\n男性,\"man\"\n猫耳の少女,\"cat ears girl\"\nエルフの少女,\"elf girl\"\nちび女の子,\"chibi girl\"\nメイド,\"maid\"\n美少女,\"beautiful girl\"\nアイドル,\"idol\"\n女子学生,\"schoolgirl\"\n女子大生,\"college student\"\n巫女,\"miko\"\nナース,\"nurse\"\n女騎士,\"female knight\"\n魔法少女,\"magical girl\"\nギャル,\"gyaru\"\nツンデレ少女,\"tsundere girl\"\nお姉さん,\"onee-san\"\nお嬢様,\"ojou-sama\"\n男の娘,\"otokonoko\""),
    ("expression.csv", "Japanese,Prompt\n笑顔,\"smile\"\n微睡み/穏やかな表情,\"gentle smile\"\n爆笑,\"laughing\"\n怒り,\"angry\"\n泣き顔,\"crying\"\n照れ/赤面,\"blushing\"\nドヤ顔,\"smug\"\n驚き,\"surprised\"\nジト目,\"jitome\"\nウィンク,\"wink\"\n悲しい顔,\"sad\"\n真剣な表情,\"serious look\"\n無表情,\"expressionless\"\n悪巧み/ニヤリ,\"grin\"\nあくび,\"yawning\"\nドヤ笑顔,\"smug smile\"\n困り顔,\"troubled face\"\nジト目笑顔,\"jitome smile\"\nジト目怒り,\"jitome angry\"\n泣き笑い,\"crying smile\"\nキス顔,\"kiss face\"\nウインク笑顔,\"wink smile\"\n目を閉じる,\"closed eyes\""),
    ("pose.csv", "Japanese,Prompt\n立ち姿,\"standing\"\n座り姿,\"sitting\"\n寝そべる,\"lying down\"\n腕組み,\"arms crossed\"\nピースサイン,\"peace sign\"\n後ろ姿,\"from behind\"\n手を振る,\"waving hand\"\n振り返る,\"looking back\"\nしゃがみ姿,\"squatting\"\nジャンプ,\"jumping\"\n頬杖をつく,\"leaning on hand\"\n腰に手を当てる,\"hand on hip\"\n走る,\"running\"\n歩く,\"walking\"\n指差し,\"pointing\"\n背伸び,\"stretching\"\nしゃがんで見上げる,\"squatting, looking up\"\n片足立ち,\"standing on one leg\"\n両手を上げる,\"arms up\"\nうつ伏せ,\"lying on stomach\"\n仰向け,\"lying on back\"\n体育座り,\"sitting with knees held\""),
    ("hair.csv", "Japanese,Prompt\nロングヘア,\"long hair\"\nショートヘア,\"short hair\"\nツインテール,\"twintails\"\nポニーテール,\"ponytail\"\nボブカット,\"bob cut\"\nお団子頭,\"hair bun\"\nブレイド/三つ編み,\"braided hair\"\n金髪,\"blonde hair\"\n黒髪,\"black hair\"\n茶髪,\"brown hair\"\n銀髪,\"silver hair\"\n白髪,\"white hair\"\n青髪,\"blue hair\"\nピンク髪,\"pink hair\"\n赤髪,\"red hair\"\n緑髪,\"green hair\"\n紫髪,\"purple hair\"\nインナーカラー,\"inner color\"\nグラデーションヘア,\"gradient hair\"\nストレートヘア,\"straight hair\"\nウェーブヘア,\"wavy hair\"\nショートボブ,\"short bob\"\nサイドテール,\"side tail\"\nアホ毛,\"ahoge\""),
    ("situation.csv", "Japanese,Prompt\n読書,\"reading book\"\n食事,\"eating\"\n睡眠/居眠り,\"sleeping\"\n勉強/執筆,\"studying\"\n音楽を聴く,\"listening to music\"\n歌う,\"singing\"\n料理,\"cooking\"\n買い物,\"shopping\"\n雨の中を歩く,\"walking in the rain\"\nスポーツ,\"playing sports\"\nおしゃべり,\"talking\"\nスマホを操作する,\"using smartphone\"\n紅茶を飲む,\"drinking tea\"\nダンス,\"dancing\"\n自撮り,\"selfie\"\n泣いている人を慰める,\"comforting\"\n写真を撮る,\"taking photo\"\n絵を描く,\"painting\"\n泳ぐ,\"swimming\"\n空を見上げる,\"looking at sky\"\n窓の外を見る,\"looking out window\"\nうつむく,\"looking down\""),
    ("clothing_real.csv", "Japanese,Prompt\n制服/セーラー服,\"school uniform, sailor uniform\"\nTシャツ,\"T-shirt\"\nジーンズ/ジーパン,\"jeans\"\nパーカー,\"hoodie\"\nビジネススーツ,\"business suit\"\nカジュアルウェア,\"casual clothes\"\nセーター,\"sweater\"\nコート,\"coat\"\nパジャマ,\"pyjamas\"\nスポーツウェア,\"sportswear\"\n夏服,\"summer clothes\"\n冬服,\"winter clothes\"\nロングスカート,\"long skirt\"\nミニスカート,\"miniskirt\"\nブレザー制服,\"blazer school uniform\"\nカーディガン,\"cardigan\"\nオフショルダートップス,\"off-shoulder top\"\nタンクトップ,\"tank top\"\nショートパンツ,\"short shorts\"\nワンピース,\"one-piece dress\"\nトレンチコート,\"trench coat\"\nダウンジャケット,\"down jacket\""),
    ("clothing_fantasy.csv", "Japanese,Prompt\n鎧/アーマー,\"armor\"\n魔法使いの服/ローブ,\"wizard robe\"\nメイド服,\"maid outfit\"\n着物,\"kimono\"\nゴスロリドレス,\"gothic lolita fashion\"\nナース服,\"nurse uniform\"\n巫女服,\"miko outfit\"\nドレス,\"elegant dress\"\nチャイナドレス,\"cheongsam\"\nバニースーツ,\"bunny suit\"\n冒険者の服,\"adventurer outfit\"\nプリンセスドレス,\"princess dress\"\nビキニアーマー,\"bikini armor\"\nサンタ服,\"santa costume\"\nシスター服,\"sister uniform\"\n浴衣,\"yukata\"\n振袖,\"furisode\"\nミリタリー制服,\"military uniform\"\n妖精のドレス,\"fairy dress\"\nサイバーパンクスーツ,\"cyberpunk suit\"\nファンタジードレス,\"fantasy dress\"\n甲冑,\"plate armor\""),
    ("accessories.csv", "Japanese,Prompt\n眼鏡,\"glasses\"\nリボン,\"ribbon\"\nチョーカー,\"choker\"\nネックレス,\"necklace\"\nイヤリング/ピアス,\"earrings\"\nヘアピン,\"hairpin\"\n帽子,\"hat\"\n王冠/ティアラ,\"crown\"\n指輪,\"ring\"\nネクタイ,\"necktie\"\nマフラー,\"scarf\"\n手袋,\"gloves\"\nヘッドホン,\"headphones\"\n腕時計,\"wrist watch\"\nベール,\"veil\"\nベルト,\"belt\"\nアンクレット,\"anklet\"\nブレスレット,\"bracelet\"\nカチューシャ,\"headband\"\n狐のお面,\"fox mask\"\nガスマスク,\"gas mask\"\nヘアバンド,\"hair band\""),
    ("background.csv", "Japanese,Prompt\n教室,\"classroom\"\n砂浜/ビーチ,\"beach\"\n森林/森,\"forest\"\n都会の街並み,\"city street\"\n夜空と星,\"night sky, stars\"\nカフェ,\"cafe\"\n図書館,\"library\"\nファンタジーの世界,\"fantasy world\"\n和室,\"japanese style room\"\n廃墟,\"ruins\"\n公園,\"park\"\n青空と雲,\"blue sky, clouds\"\n部屋/寝室,\"bedroom\"\n宇宙,\"outer space\"\n夕暮れの街,\"sunset street\"\nお城/宮殿,\"castle, palace\"\n山,\"mountains\"\n雪景色,\"snow scene\"\nネオン輝くサイバーパンク街,\"cyberpunk city lights, neon\"\n水中,\"underwater\"\n花畑,\"flower field\"\n洋館,\"western-style mansion\""),
    ("effect.csv", "Japanese,Prompt\n被写界深度/背景ぼかし,\"depth of field, blurry background\"\nレンズフレア,\"lens flare\"\n逆光,\"backlighting\"\n光の粒子,\"light particles\"\nキラキラ/グリッター,\"glittering\"\nネオン光,\"neon glow\"\nシネマティックライティング,\"cinematic lighting\"\nソフトフォーカス,\"soft focus\"\n影の演出,\"dramatic shadows\"\n水滴/雨のエフェクト,\"water drops, rain effect\"\n風の演出,\"wind blow\"\nカラフルな光,\"colorful light\"\n光の線,\"light streaks\"\n煙/スモーク,\"smoke, haze\"\nゴッドレイ/光の差し込み,\"god rays, sun beams\"\n炎のエフェクト,\"fire effect\"\n氷のエフェクト,\"ice effect\"\n電気/雷エフェクト,\"lightning effect\"\n魔法陣/スペルエフェクト,\"magic circle, spell effect\"\n水しぶき,\"water splash\"\n桜吹雪,\"cherry blossoms scattering\"\n羽が舞う,\"falling feathers\""),
    ("n_body_type.csv", "Japanese,Prompt\n巨乳,\"large breasts\"\n貧乳/ちっぱい,\"small breasts, flat chest\"\n爆乳,\"huge breasts\"\nぽっちゃり,\"chubby, plump\"\nスレンダー/細身,\"slender, slim\"\nむっちり/肉感的な体,\"thick thighs, curvy\"\n筋肉質/腹筋,\"muscular, female muscle, abs\"\nくびれ,\"hourglass figure, narrow waist\"\n巨尻/大きなお尻,\"huge ass, large hips\"\n低身長/小柄,\"petite, short stature\"\n高身長,\"tall female\"\n妊娠/妊婦,\"pregnant\"\n巨乳化,\"breast expansion\"\nふたなり,\"futanari\""),
    ("n_nsfw.csv", "Japanese,Prompt\nNSFW（成人向け表示）,\"nsfw\"\nヌード/裸,\"nudity, naked\"\nトップレス/上半身裸,\"topless\"\nボトムレス/下半身裸,\"bottomless\"\nモザイクなし,\"uncensored\"\n半裸,\"semi-nudity\"\n露出度の高い服,\"revealing clothes\"\n完全ヌード,\"completely naked, fully nude\"\n局部露出,\"genitals exposed\"\nパンスト越しの透け,\"panties under pantyhose, sheer clothing\"\n下着姿,\"lingerie, underwear\"\n水着姿,\"swimwear, bikini\"\nマイクロビキニ,\"micro bikini\"\n透け透けの服,\"see-through clothing\""),
    ("n_expression.csv", "Japanese,Prompt\nアヘ顔,\"ahegao\"\n舌出し,\"tongue out\"\n恍惚とした表情,\"ecstasy\"\n羞恥/恥ずかしそうな顔,\"embarrassed\"\n淫らな笑顔,\"lewd smile\"\n息が荒い,\"heavy breathing\"\nキス待ちの顔,\"waiting for kiss\"\nよだれ/涎,\"drooling\"\n蕩けた目,\"dilated pupils, dreamy eyes\"\nトランス状態,\"trance\"\n媚びるような目,\"seductive look\"\n悶える表情,\"pained smile\"\n喘ぎ顔,\"orgasm face\"\nハート目,\"heart-shaped pupils\""),
    ("n_accessories.csv", "Japanese,Prompt\n首輪,\"collar\"\n手錠,\"handcuffs\"\n目隠し,\"blindfold\"\n縄/緊縛,\"rope, bondage\"\nニップルピアス,\"nipple piercing\"\nガーターベルト,\"garter belt\"\n口枷,\"gag\"\n拘束衣,\"straitjacket\"\n鎖,\"chains\"\n性玩具/バイブ,\"sex toy, vibrator\"\n乳首クリップ,\"nipple clamps\"\nアナルプラグ,\"anal plug\"\nアイマスク,\"eye mask\"\n紐パン,\"side-tie panties\""),
    ("n_pose.csv", "Japanese,Prompt\n股を開く,\"legs spread\"\n前屈み/お尻を向ける,\"bent over, presenting\"\nM字開脚,\"m-leg split\"\n四つん這い,\"all fours\"\n寝そべって股を開く,\"lying on back, legs spread\"\n胸を強調する,\"breast squeeze\"\nお尻を強調する,\"ass focus\"\n脚を組む,\"legs crossed\"\n股間のクローズアップ,\"crotch close-up\"\n胸のクローズアップ,\"breast close-up\"\n腰を振る,\"hip shake\"\n自ら服をめくりあげる,\"shirt lift, skirt lift\"\nバックショット/後ろから,\"from behind, backshot\""),
    ("n_situation.csv", "Japanese,Prompt\n服を脱ぐ/脱衣,\"undressing, stripping\"\n服を破られる,\"ripped clothes\"\nシャワーを浴びる,\"shivering in shower\"\n胸を揉まれる,\"groping\"\n愛撫,\"caressing\"\nお風呂,\"in bath\"\nセクシャルなマッサージ,\"sexual massage\"\n潮吹き,\"squirt\"\n射精される,\"cum inside, cum on body\"\n顔射,\"facial cum\"\n乳房の間で挟む/パイズリ,\"paizuri, breast stimulation\"\n手コキ,\"handjob\"\nフェラチオ,\"fellatio, blowjob\"\nクンニリングス,\"cunnilingus\"\nセックス/挿入,\"sex, vaginal penetration\"\nバックシチュエーション,\"doggystyle, penetration from behind\"\n中出し/膣内射精,\"creampie, vaginal cum\"\nアナル中出し/アナル射精,\"anal creampie, anal cum\"\n生セックス/生ハメ,\"bareback, raw sex\"\nコンドーム着用,\"condom, wearing condom\"\nペニス/勃起,\"penis, erect penis\"\n竿握り/ペニスを握る,\"holding penis\"\nイラマチオ/深口工,\"irrumatio, deepthroat\"\nダブル挿入/2本挿し,\"double penetration\"\n射精/放精,\"ejaculation, cumming\""),
    ("negative_prompt.csv", "Japanese,Prompt\n基本ネガティブセット,\"lowres, bad anatomy, bad hands, text, error, missing fingers, extra digit, fewer digits, cropped, worst quality, low quality, normal quality, jpeg artifacts, signature, watermark, username, blurry\"\n低クオリティ除外,\"worst quality, low quality, normal quality, lowres\"\n奇形手足除外,\"bad anatomy, bad hands, missing fingers, extra digit, fewer digits, mutated hands, poorly drawn hands, poorly drawn face, mutation, deformed\"\nテキスト・文字除外,\"text, watermark, signature, username, logo, words, letters\"\n崩れた顔除外,\"deformed iris, deformed pupils, bad eyes, poorly drawn eyes, bad face, poorly drawn face\"\n変なポーズ・崩れ除外,\"bad proportions, extra limbs, cloned face, disfigured, gross proportions, malformed limbs, missing arms, missing legs, extra arms, extra legs, mutated hands\"\nNSFW要素の排除(SFW設定),\"nsfw, nudity, naked, nipples, vaginal, pubic hair, breast squeeze, adult content, suggestive\""),
];

// ==========================================================================
// Category Title Helpers
// ==========================================================================
fn get_friendly_name(filename: &str) -> String {
    let base = filename.trim_end_matches(".csv");
    match base {
        "quality" => "品質".to_string(),
        "art_style" => "画風".to_string(),
        "subject" => "人物".to_string(),
        "expression" => "表情".to_string(),
        "pose" => "ポーズ".to_string(),
        "hair" => "髪".to_string(),
        "situation" => "シチュエーション".to_string(),
        "clothing_real" => "服装 - リアル".to_string(),
        "clothing_fantasy" => "服装 - ファンタジー".to_string(),
        "accessories" => "アクセサリー".to_string(),
        "background" => "背景".to_string(),
        "effect" => "効果".to_string(),
        "n_body_type" => "NSFW 体形".to_string(),
        "n_nsfw" => "NSFW 基本設定".to_string(),
        "n_expression" => "NSFW 表情".to_string(),
        "n_accessories" => "NSFW アクセサリー".to_string(),
        "n_pose" => "NSFW ポーズセット".to_string(),
        "n_situation" => "NSFW シチュエーション".to_string(),
        "negative_prompt" => "ネガティブプロンプトセット".to_string(),
        // Fallback for custom added files
        other => {
            // Remove 'n_' prefix if any
            let clean = if other.starts_with("n_") {
                &other[2..]
            } else {
                other
            };
            // Replace underscores with spaces and capitalize
            clean.split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    }
}

// ==========================================================================
// CSV Parsing Logic
// ==========================================================================
fn parse_csv_file(path: &Path) -> Result<Vec<PromptItem>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut items = Vec::new();
    let mut lines = content.lines();
    
    // Skip header
    let _header = lines.next();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        
        for char in line.chars() {
            if char == '"' {
                in_quotes = !in_quotes;
            } else if char == ',' && !in_quotes {
                parts.push(current.trim().to_string());
                current.clear();
            } else {
                current.push(char);
            }
        }
        parts.push(current.trim().to_string());
        
        if parts.len() >= 2 {
            let jp = parts[0].trim_matches('"').trim().to_string();
            let en = parts[1].trim_matches('"').trim().to_string();
            
            let jp_normalized = jp.replace('／', "/");
            if jp_normalized.contains('/') {
                let jp_parts: Vec<String> = jp_normalized.split('/')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                let en_parts: Vec<String> = en.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if jp_parts.len() == en_parts.len() {
                    for (j_part, e_part) in jp_parts.into_iter().zip(en_parts.into_iter()) {
                        items.push(PromptItem { japanese: j_part, prompt: e_part });
                    }
                } else {
                    items.push(PromptItem { japanese: jp, prompt: en });
                }
            } else {
                items.push(PromptItem { japanese: jp, prompt: en });
            }
        }
    }
    
    Ok(items)
}

// ==========================================================================
// CSV Path Helper
// ==========================================================================
fn get_csv_dir() -> std::path::PathBuf {
    let pwd_csv = std::path::Path::new("csv");
    if pwd_csv.exists() {
        return pwd_csv.to_path_buf();
    }
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("csv");
        if exe_path.exists() {
            return exe_path;
        }
        if std::env::var("CARGO_MANIFEST_DIR").is_err() {
            return exe_path;
        }
    }
    pwd_csv.to_path_buf()
}

// ==========================================================================
// Favorites Path Helper
// ==========================================================================
fn get_favorites_dir() -> std::path::PathBuf {
    let pwd_fav = std::path::Path::new("favorites");
    if pwd_fav.exists() {
        return pwd_fav.to_path_buf();
    }
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("favorites");
        if exe_path.exists() {
            return exe_path;
        }
        if std::env::var("CARGO_MANIFEST_DIR").is_err() {
            return exe_path;
        }
    }
    pwd_fav.to_path_buf()
}

fn init_favorites_directory() -> Result<(), std::io::Error> {
    let fav_dir = get_favorites_dir();
    if !fav_dir.exists() {
        println!("Creating favorites directory at {:?}", fav_dir);
        fs::create_dir(&fav_dir)?;
    }
    Ok(())
}

// ==========================================================================
// Directory Init (Bootstrap)
// ==========================================================================
fn init_csv_directory() -> Result<(), std::io::Error> {
    let csv_dir = get_csv_dir();
    if !csv_dir.exists() {
        println!("Creating csv directory at {:?} and writing default CSV files...", csv_dir);
        fs::create_dir(&csv_dir)?;
        
        for &(filename, content) in DEFAULT_CSVS {
            let file_path = csv_dir.join(filename);
            let mut file = File::create(file_path)?;
            file.write_all(content.as_bytes())?;
        }
    }
    Ok(())
}

// ==========================================================================
// Handlers
// ==========================================================================
async fn get_prompts() -> Json<PromptsResponse> {
    let mut sfw = Vec::new();
    let mut nsfw = Vec::new();
    let mut negative = Vec::new();
    
    let csv_dir = get_csv_dir();
    if let Ok(entries) = fs::read_dir(&csv_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "csv") {
                if let Some(filename_os) = path.file_name() {
                    let filename = filename_os.to_string_lossy().to_string();
                    if let Ok(tags) = parse_csv_file(&path) {
                        let name = get_friendly_name(&filename);
                        let group = CategoryGroup {
                            name,
                            filename: filename.to_string(),
                            tags,
                        };
                        
                        // Classify based on filename rules
                        if filename.starts_with("n_") {
                            nsfw.push(group);
                        } else if filename.contains("negative") {
                            negative.push(group);
                        } else {
                            sfw.push(group);
                        }
                    }
                }
            }
        }
    }
    
    // Sort SFW categories according to original design priority
    let priority_order = [
        "quality.csv", "art_style.csv", "subject.csv", "expression.csv",
        "pose.csv", "hair.csv", "situation.csv", "clothing_real.csv",
        "clothing_fantasy.csv", "accessories.csv", "background.csv", "effect.csv"
    ];
    
    let mut sfw_map: HashMap<String, CategoryGroup> = sfw.into_iter()
        .map(|item| (item.filename.clone(), item))
        .collect();
        
    let mut sorted_sfw = Vec::new();
    for &filename in &priority_order {
        if let Some(group) = sfw_map.remove(filename) {
            sorted_sfw.push(group);
        }
    }
    // Append any remaining custom SFW files
    for (_, group) in sfw_map {
        sorted_sfw.push(group);
    }
    
    // Sort NSFW categories according to design priority
    let nsfw_priority_order = [
        "n_nsfw.csv", "n_body_type.csv", "n_expression.csv",
        "n_pose.csv", "n_situation.csv", "n_accessories.csv"
    ];
    
    let mut nsfw_map: HashMap<String, CategoryGroup> = nsfw.into_iter()
        .map(|item| (item.filename.clone(), item))
        .collect();
        
    let mut sorted_nsfw = Vec::new();
    for &filename in &nsfw_priority_order {
        if let Some(group) = nsfw_map.remove(filename) {
            sorted_nsfw.push(group);
        }
    }
    // Append any remaining custom NSFW files
    for (_, group) in nsfw_map {
        sorted_nsfw.push(group);
    }
    
    Json(PromptsResponse {
        sfw: sorted_sfw,
        nsfw: sorted_nsfw,
        negative,
    })
}

// ==========================================================================
// Favorites Handlers
// ==========================================================================
async fn get_favorites() -> Result<Json<Vec<FavoritePreset>>, StatusCode> {
    let fav_dir = get_favorites_dir();
    if !fav_dir.exists() {
        return Ok(Json(Vec::new()));
    }
    
    let mut presets = Vec::new();
    if let Ok(entries) = fs::read_dir(&fav_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(preset) = serde_json::from_str::<FavoritePreset>(&content) {
                        presets.push(preset);
                    }
                }
            }
        }
    }
    
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(presets))
}

async fn save_favorite(Json(preset): Json<FavoritePreset>) -> Result<StatusCode, StatusCode> {
    let fav_dir = get_favorites_dir();
    if !fav_dir.exists() {
        if let Err(e) = fs::create_dir(&fav_dir) {
            eprintln!("Failed to create favorites dir: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    let name_trimmed = preset.name.trim();
    if name_trimmed.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let safe_filename: String = name_trimmed.chars()
        .filter(|&c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'))
        .collect();
        
    let safe_filename = safe_filename.trim();
    if safe_filename.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let filename = format!("{}.json", safe_filename);
    let path = fav_dir.join(filename);
    
    match serde_json::to_string_pretty(&preset) {
        Ok(json_str) => {
            if let Err(e) = fs::write(path, json_str) {
                eprintln!("Failed to write favorite file: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            } else {
                Ok(StatusCode::CREATED)
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_favorite(axum::extract::Path(name): axum::extract::Path<String>) -> Result<StatusCode, StatusCode> {
    let fav_dir = get_favorites_dir();
    let name_trimmed = name.trim();
    if name_trimmed.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let safe_filename: String = name_trimmed.chars()
        .filter(|&c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'))
        .collect();
        
    let safe_filename = safe_filename.trim();
    if safe_filename.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let filename = format!("{}.json", safe_filename);
    let path = fav_dir.join(filename);
    
    if path.exists() && path.is_file() {
        if let Err(e) = fs::remove_file(path) {
            eprintln!("Failed to delete favorite file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            Ok(StatusCode::OK)
        }
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Handler serving static embedded files
async fn static_handler(path: &str) -> impl IntoResponse {
    let path = if path.is_empty() || path == "/" { "index.html" } else { path.trim_start_matches('/') };
    
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

// ==========================================================================
// Heartbeat / Shutdown Logic
// ==========================================================================
static LAST_PING: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn get_last_ping() -> &'static Mutex<Option<Instant>> {
    LAST_PING.get_or_init(|| Mutex::new(None))
}

async fn ping() -> impl IntoResponse {
    let mut last_ping = get_last_ping().lock().unwrap();
    *last_ping = Some(Instant::now());
    StatusCode::OK
}

// ==========================================================================
// Main Runner
// ==========================================================================
#[tokio::main]
async fn main() {
    // 1. Bootstrap default CSV files if folder not present
    if let Err(e) = init_csv_directory() {
        eprintln!("Failed to initialize CSV directory: {}", e);
        return;
    }
    if let Err(e) = init_favorites_directory() {
        eprintln!("Failed to initialize favorites directory: {}", e);
    }
    
    // 2. Build Axum Router
    let app = Router::new()
        .route("/api/prompts", get(get_prompts))
        .route("/api/ping", get(ping))
        .route("/api/favorites", get(get_favorites).post(save_favorite))
        .route("/api/favorites/:name", delete(delete_favorite))
        .fallback(get(|uri: axum::http::Uri| async move {
            static_handler(uri.path()).await
        }));
        
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Starting AnimaI T2I server on http://localhost:8080 ...");
    
    // 3. Open Web Browser
    let browser_addr = "http://localhost:8080";
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Err(e) = webbrowser::open(browser_addr) {
            eprintln!("Failed to open default web browser: {}", e);
        }
    });
    
    // 3.5. Start ping checker task
    tokio::spawn(async move {
        // Wait a bit before starting to check, to give the browser time to open and connect
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let last = *get_last_ping().lock().unwrap();
            if let Some(time) = last {
                if time.elapsed() > std::time::Duration::from_secs(5) {
                    println!("ブラウザのタブが閉じられたため、サーバーを終了します...");
                    std::process::exit(0);
                }
            }
        }
    });

    // 4. Bind & Run HTTP Server
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("\n============================================================");
            eprintln!("【エラー】起動に失敗しました。");
            eprintln!("原因: {}", e);
            eprintln!("既にこのアプリが起動しているか、他のアプリがポート8080を使用しています。");
            eprintln!("他の黒い画面（コマンドプロンプト）が開いている場合は、すべて閉じてから再度お試しください。");
            eprintln!("============================================================\n");
            std::process::exit(1);
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("サーバーエラー: {}", e);
    }
}
