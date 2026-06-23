// tests/readme_snippets.rs

use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

fn extract_code_snippets(readme_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let readme_content = fs::read_to_string(readme_path)?;

    let code_block_pattern = Regex::new(r"(?s)```(?:rust|rs)[^\n\r]*\r?\n(.*?)\r?\n```")?;

    let snippets: Vec<String> = code_block_pattern
        .captures_iter(&readme_content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    Ok(snippets.into_iter().collect())
}

fn extract_example_code(example_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(example_path)?)
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn split_first_line(s: &str) -> Option<(&str, &str)> {
    if let Some(idx) = s.find('\n') {
        let first = &s[..idx];
        let rest = &s[idx + 1..];
        Some((first, rest))
    } else {
        None
    }
}

#[test]
fn test_readme_snippets() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let readme_path = ["README.md", "readme.md"]
        .into_iter()
        .map(|name| project_root.join(name))
        .find(|p| p.exists())
        .ok_or("README file not found at README.md or readme.md")?;

    let snippets = extract_code_snippets(&readme_path)?;

    if snippets.is_empty() {
        return Err(format!(
            "No Rust code snippets found in README: {}",
            readme_path.display()
        )
        .into());
    }

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for (i, snippet) in snippets.iter().enumerate() {
        let snippet_index = i + 1;

        let normalized_snippet = normalize_newlines(snippet);

        let (first_line, snippet_without_comment) = match split_first_line(&normalized_snippet) {
            Some(parts) => parts,
            None => {
                errors.push(format!(
                    "Snippet {}: Empty or single-line snippet found",
                    snippet_index
                ));
                continue;
            }
        };

        let first_line = first_line.trim();

        if !first_line.starts_with("//") {
            errors.push(format!(
                "Snippet {}: First line is not a comment with file path: {}",
                snippet_index, first_line
            ));
            continue;
        }

        let example_file_name = first_line.trim_start_matches("//").trim();

        if example_file_name.is_empty() {
            errors.push(format!(
                "Snippet {}: Could not extract filename from comment: {}",
                snippet_index, first_line
            ));
            continue;
        }

        let example_path = project_root.join(example_file_name);

        if !example_path.exists() {
            errors.push(format!(
                "Snippet {}: Example file not found: {}",
                snippet_index,
                example_path.display()
            ));
            continue;
        }

        let example_code = match extract_example_code(&example_path) {
            Ok(code) => normalize_newlines(&code),
            Err(e) => {
                errors.push(format!(
                    "Snippet {}: Failed to read {}: {}",
                    snippet_index, example_file_name, e
                ));
                continue;
            }
        };

        if snippet_without_comment.trim().is_empty() {
            errors.push(format!(
                "Snippet {}: Snippet is empty after removing comment line",
                snippet_index
            ));
            continue;
        }

        let is_exact_match = snippet_without_comment == example_code;
        results.push((example_file_name.to_string(), is_exact_match));

        if !is_exact_match {
            let snippet_lines: Vec<&str> = snippet_without_comment.split('\n').collect();
            let example_lines: Vec<&str> = example_code.split('\n').collect();

            let mut found_diff = false;

            for (line_num, (s_line, e_line)) in
                snippet_lines.iter().zip(example_lines.iter()).enumerate()
            {
                if s_line != e_line {
                    errors.push(format!(
                        "Snippet {} ({}): Mismatch at line {}\n    README: {:?}\n    Example: {:?}",
                        snippet_index,
                        example_file_name,
                        line_num + 1,
                        s_line,
                        e_line
                    ));
                    found_diff = true;
                    break;
                }
            }

            if !found_diff {
                errors.push(format!(
                    "Snippet {} ({}): Different number of lines\n    README: {} lines\n    Example: {} lines\n    README ends with newline: {}\n    Example ends with newline: {}",
                    snippet_index,
                    example_file_name,
                    snippet_lines.len(),
                    example_lines.len(),
                    snippet_without_comment.ends_with('\n'),
                    example_code.ends_with('\n'),
                ));
            }
        }
    }

    println!("\nProcessed {} code snippets from README\n", snippets.len());

    for (example_file_name, result) in &results {
        let status = if *result { "PASSED" } else { "FAILED" };
        println!("  {}: {}", status, example_file_name);
    }

    if !errors.is_empty() {
        println!("\n{}", "=".repeat(80));
        println!("ERRORS FOUND ({}):", errors.len());
        println!("{}", "=".repeat(80));
        for error in &errors {
            println!("\n{}", error);
        }
        println!("{}", "=".repeat(80));

        panic!(
            "Found {} error(s) in README code snippets. Fix the README to match example files exactly.",
            errors.len()
        );
    }

    println!(
        "\nSUCCESS! All {} README code snippets match their example files exactly!",
        results.len()
    );

    Ok(())
}
