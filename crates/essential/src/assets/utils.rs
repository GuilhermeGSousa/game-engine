use cfg_if::cfg_if;

use super::AssetPath;

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("learn-wgpu") {
        origin = format!("{}/learn-wgpu", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_to_string(path: AssetPath) -> Result<String, ()> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(path);
            let txt = reqwest::get(url)
                .await?
                .text()
                .await?;
        } else {
            let path = std::env::current_exe().unwrap().parent().unwrap()
                .join(path.normalized_path);
            let txt = std::fs::read_to_string(&path).map_err(|_|
                {
                    println!("Failed to load file: {}", &path.display());
                    ()
                })?;
        }
    }

    Ok(txt)
}

pub async fn load_binary(path: AssetPath) -> Result<Vec<u8>, ()> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(path);
            let data = reqwest::get(url)
                .await?
                .bytes()
                .await?
                .to_vec();
        } else {
            let path = std::env::current_exe().unwrap().parent().unwrap()
                .join(path.normalized_path);
            let data = std::fs::read(&path).map_err(|_|
                {
                    println!("Failed to load file: {}", &path.display());
                    ()
                })?;
        }
    }

    Ok(data)
}
