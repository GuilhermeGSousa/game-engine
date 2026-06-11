use anyhow::Context;
use cfg_if::cfg_if;

use super::AssetPath;

#[cfg(target_arch = "wasm32")]
fn format_url<'a>(path: AssetPath<'a>) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let origin = location.origin().unwrap();

    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(path.to_string()).unwrap()
}

pub async fn load_to_string<'a>(path: AssetPath<'a>) -> anyhow::Result<String> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(path);
            let txt = reqwest::get(url.clone())
                .await
                .with_context(|| format!("HTTP request for asset '{}' failed", url))?
                .text()
                .await
                .with_context(|| format!("failed to read response body for asset '{}'", url))?;
        } else {
            let exe_path = std::env::current_exe()
                .context("could not determine executable path")?;
            let exe_dir = exe_path
                .parent()
                .context("could not determine executable directory")?;
            let path = exe_dir.join(path.normalized_path);
            let txt = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read file '{}'", path.display()))?;
        }
    }

    Ok(txt)
}

pub async fn load_binary<'a>(path: AssetPath<'a>) -> anyhow::Result<Vec<u8>> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(path);
            let data = reqwest::get(url.clone())
                .await
                .with_context(|| format!("HTTP request for asset '{}' failed", url))?
                .bytes()
                .await
                .with_context(|| format!("failed to read response body for asset '{}'", url))?
                .to_vec();
        } else {
            let exe_path = std::env::current_exe()
                .context("could not determine executable path")?;
            let exe_dir = exe_path
                .parent()
                .context("could not determine executable directory")?;
            let path = exe_dir.join(path.normalized_path);
            let data = std::fs::read(&path)
                .with_context(|| format!("failed to read file '{}'", path.display()))?;
        }
    }

    Ok(data)
}
