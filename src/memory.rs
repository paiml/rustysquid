use std::fs;
use tracing::debug;

/// Check if system has enough memory for caching
/// Returns true if caching should proceed, false if memory is low
pub fn has_sufficient_memory() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            let mut mem_available = 0;
            let mut mem_total = 0;

            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        mem_available = kb_str.parse::<usize>().unwrap_or(0);
                    }
                } else if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        mem_total = kb_str.parse::<usize>().unwrap_or(0);
                    }
                }
            }

            // Need at least 100MB available or 10% of total memory
            let min_available = 100 * 1024; // 100MB in KB
            let min_percent = mem_total / 10; // 10% of total
            let required = min_available.max(min_percent);

            let sufficient = mem_available > required;
            debug!(
                "Memory check: available={}MB, required={}MB, sufficient={}",
                mem_available / 1024,
                required / 1024,
                sufficient
            );
            return sufficient;
        }
    }

    // Default to true if we can't check
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_check() {
        // Should always return a boolean
        let _result = has_sufficient_memory();
        // Function always returns a boolean by definition
    }
}
