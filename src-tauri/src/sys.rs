//! Petits outils communs à Windows et Linux : lancer un programme sans fenêtre noire,
//! découper une ligne de commande, nommer un fichier quel que soit le séparateur.

use std::path::Path;
use std::process::{Command, Stdio};

/// Un `Command` prêt à l'emploi. Sous Windows, aucune fenêtre de console ne s'ouvre.
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Lance un programme et le laisse vivre sa vie (on n'attend pas qu'il se ferme).
pub fn spawn(program: &str, args: &[&str]) -> Result<(), String> {
    let mut c = command(program);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(dir) = Path::new(program).parent().filter(|d| d.is_dir()) {
        c.current_dir(dir);
    }
    c.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Lance un programme, attend la fin et renvoie ce qu'il a écrit (vide si échec).
pub fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = command(program).args(args).stdin(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Le nom à afficher pour un chemin, avec `/` ou `\` comme séparateur.
pub fn file_name(path: &str) -> String {
    let p = path.trim_end_matches(['/', '\\']);
    if p.is_empty() {
        return path.to_string();
    }
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

/// Découpe une ligne de commande Windows comme le fait Windows lui-même (guillemets,
/// barres obliques inversées devant un guillemet).
pub fn split_windows_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_arg = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                let mut n = 0;
                while i < chars.len() && chars[i] == '\\' {
                    n += 1;
                    i += 1;
                }
                if i < chars.len() && chars[i] == '"' {
                    cur.extend(std::iter::repeat('\\').take(n / 2));
                    if n % 2 == 1 {
                        cur.push('"');
                        i += 1;
                    }
                } else {
                    cur.extend(std::iter::repeat('\\').take(n));
                }
                has_arg = true;
                continue;
            }
            '"' => {
                if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                    cur.push('"');
                    i += 1;
                } else {
                    in_quotes = !in_quotes;
                }
                has_arg = true;
            }
            ' ' | '\t' if !in_quotes => {
                if has_arg {
                    args.push(std::mem::take(&mut cur));
                    has_arg = false;
                }
            }
            _ => {
                cur.push(c);
                has_arg = true;
            }
        }
        i += 1;
    }
    if has_arg {
        args.push(cur);
    }
    args
}

/// Petit encodage base64 (pour passer un script à PowerShell sans souci de guillemets).
pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Ce chemin ressemble-t-il à un fichier de l'utilisateur à rouvrir (et pas à un morceau
/// de programme) ? On ne garde que ce qui existe et qui est dans le dossier personnel
/// ou sur un disque externe.
pub fn is_user_path(path: &str, home: &str) -> bool {
    if path.len() < 3 || path.starts_with('-') {
        return false;
    }
    let p = Path::new(path);
    if !p.is_absolute() || !p.exists() {
        return false;
    }
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let home = home.to_ascii_lowercase().replace('\\', "/");
    let inside_home = !home.is_empty() && lower.starts_with(&home);
    let removable = lower.starts_with("/media/") || lower.starts_with("/mnt/") || lower.starts_with("/run/media/");
    let hidden_bits = ["/appdata/", "/.cache/", "/.local/share/", "/.config/", "/.var/", "/snap/", "/.mozilla/", "/node_modules/"];
    (inside_home || removable) && !hidden_bits.iter().any(|h| lower.contains(h)) && lower != home.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_args() {
        assert_eq!(file_name("C:\\Users\\Moi\\Rapport final.docx"), "Rapport final.docx");
        assert_eq!(file_name("/home/moi/Documents/"), "Documents");
        assert_eq!(
            split_windows_args(r#""C:\Program Files\Microsoft Office\WINWORD.EXE" /n "C:\Users\Moi\Exposé.docx""#),
            vec![r"C:\Program Files\Microsoft Office\WINWORD.EXE", "/n", r"C:\Users\Moi\Exposé.docx"]
        );
        assert_eq!(split_windows_args(r#"a "b c" d\"e"#), vec!["a", "b c", "d\"e"]);
        assert_eq!(split_windows_args(r#"notepad.exe C:\x\y.txt"#), vec!["notepad.exe", r"C:\x\y.txt"]);
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }
}
