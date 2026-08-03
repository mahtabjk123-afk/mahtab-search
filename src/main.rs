use memchr::memmem;
use memmap2::Mmap;
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::time::Instant;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  search.exe --search <query> [directory]");
        eprintln!("  search.exe --ai <prompt> [corpus.txt]");
        eprintln!("  search.exe --chat [corpus.txt]");
        return Ok(());
    }

    match args[1].as_str() {
        "--search" => {
            if args.len() < 3 {
                eprintln!("Usage: search.exe --search <query> [directory]");
                return Ok(());
            }
            let query = &args[2];
            let root = if args.len() > 3 { &args[3] } else { "." };
            run_search(query, root)?;
        }
        "--ai" => {
            if args.len() < 3 {
                eprintln!("Usage: search.exe --ai <prompt> [corpus.txt]");
                return Ok(());
            }
            let prompt = &args[2];
            let corpus_file = if args.len() > 3 { &args[3] } else { "corpus.txt" };
            run_ai_single(prompt, corpus_file)?;
        }
        "--chat" => {
            let corpus_file = if args.len() > 2 { &args[2] } else { "corpus.txt" };
            run_ai_chat(corpus_file)?;
        }
        _ => eprintln!("Unknown command. Use --search, --ai, or --chat"),
    }
    Ok(())
}

fn run_search(query: &str, root: &str) -> io::Result<()> {
    let start = Instant::now();
    let root_path = Path::new(root);
    let mut found_count = 0;
    let mut total_bytes = 0;
    println!("🔍 Mahtab-Search v1.0 | Query: '{}' | Root: {}", query, root_path.display());
    println!("{}", "━".repeat(50));
    search_dir(root_path, query, &mut found_count, &mut total_bytes)?;
    let elapsed = start.elapsed();
    let mb = total_bytes as f64 / 1_000_000.0;
    let latency = elapsed.as_secs_f64() * 1000.0;
    println!("{}", "━".repeat(50));
    println!("✅ Found {} matches in {:.2} MB data | Time: {:.2} ms | Speed: {:.2} MB/s",
        found_count, mb, latency,
        if latency > 0.0 { mb / (latency / 1000.0) } else { 0.0 }
    );
    Ok(())
}

fn search_dir(path: &Path, query: &str, found_count: &mut usize, total_bytes: &mut usize) -> io::Result<()> {
    let entries = fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "target" || name == ".git" { continue; }
            }
            search_dir(&path, query, found_count, total_bytes)?;
            continue;
        }
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            match ext_str {
                "rs" | "toml" | "txt" | "md" | "json" | "yaml" | "yml" | "log" | "html"
                | "css" | "js" | "sh" | "c" | "h" | "cpp" | "go" => {}
                _ => continue,
            }
        } else { continue; }
        let file = File::open(&path)?;
        let file_size = file.metadata()?.len() as usize;
        if file_size == 0 { continue; }
        *total_bytes += file_size;
        let mmap = unsafe { Mmap::map(&file)? };
        let data = &mmap[..];
        let results = find_matches(data, query.as_bytes());
        if !results.is_empty() {
            *found_count += results.len();
            let file_name = path.display().to_string();
            for (line_num, line_content) in results.iter().take(5) {
                println!("📄 {}:{}: {}", file_name, line_num, String::from_utf8_lossy(line_content));
            }
            if results.len() > 5 {
                println!("   ... and {} more matches", results.len() - 5);
            }
        }
    }
    Ok(())
}

fn find_matches(data: &[u8], query: &[u8]) -> Vec<(usize, Vec<u8>)> {
    let mut results = Vec::new();
    let mut pos = 0;
    let mut line_num = 1;
    while pos < data.len() {
        let line_end = if let Some(newline_pos) = data[pos..].iter().position(|&b| b == b'\n') {
            newline_pos + pos
        } else { data.len() };
        let line = &data[pos..line_end];
        if memmem::find(line, query).is_some() {
            results.push((line_num, line.to_vec()));
        }
        line_num += 1;
        pos = line_end + 1;
        if pos >= data.len() { break; }
    }
    results
}

fn run_ai_single(prompt: &str, corpus_file: &str) -> io::Result<()> {
    println!("🤖 Mahtab-AI v1.0 | Offline Markov Chain");
    println!("📖 Corpus: {} | Prompt: '{}'", corpus_file, prompt);
    println!("{}", "━".repeat(50));
    let chain = build_markov_chain(corpus_file)?;
    let output = generate_text(&chain, prompt, 20);
    println!("📝 {}", output);
    println!("{}", "━".repeat(50));
    println!("✅ AI complete! (Offline, zero-dependency)");
    Ok(())
}

fn run_ai_chat(corpus_file: &str) -> io::Result<()> {
    println!("🤖 Mahtab-AI v1.0 | Interactive Chat Mode");
    println!("📖 Corpus: {}", corpus_file);
    println!("💡 Type your prompt and press Enter. Type 'exit' or 'quit' to stop.");
    println!("{}", "━".repeat(50));
    let chain = build_markov_chain(corpus_file)?;
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("🧑 You: ");
        stdout.flush()?;
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim();
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("👋 Goodbye! ALLAH hafiz.");
            break;
        }
        if input.is_empty() { continue; }
        let response = generate_text(&chain, input, 15);
        println!("🤖 AI: {}", response);
    }
    Ok(())
}

fn build_markov_chain(corpus_file: &str) -> io::Result<HashMap<String, Vec<String>>> {
    let file = File::open(corpus_file)?;
    let reader = BufReader::new(file);
    let mut words: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        for word in line.split_whitespace() {
            words.push(word.to_string());
        }
    }
    if words.len() < 3 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Corpus too small"));
    }
    let mut chain: HashMap<String, Vec<String>> = HashMap::new();
    for i in 0..words.len() - 1 {
        let current = words[i].clone();
        let next = words[i + 1].clone();
        chain.entry(current).or_insert_with(Vec::new).push(next);
    }
    Ok(chain)
}

fn generate_text(chain: &HashMap<String, Vec<String>>, prompt: &str, max_words: usize) -> String {
    let mut output = prompt.to_string();
    let mut current = prompt.to_string();
    let mut generated = 0;
    while generated < max_words {
        if let Some(next_words) = chain.get(&current) {
            if next_words.is_empty() { break; }
            let next = &next_words[0];
            output.push_str(" ");
            output.push_str(next);
            current = next.clone();
            generated += 1;
        } else {
            if let Some((first_key, _)) = chain.iter().next() {
                current = first_key.clone();
                output.push_str(" ");
                output.push_str(current.as_str());
                generated += 1;
            } else { break; }
        }
    }
    output
}
