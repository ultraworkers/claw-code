/// Session compaction module — Trident-inspired 3-stage pipeline
///
/// Reduces conversation token usage by:
/// 1. Superseding obsolete file operations
/// 2. Collapsing chatty message chains
/// 3. Clustering semantically similar messages
use std::collections::{BTreeMap, BTreeSet};

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    pub stage1_enabled: bool,
    pub stage2_enabled: bool,
    pub stage3_enabled: bool,
    pub collapse_threshold: usize,
    pub cluster_min_size: usize,
    pub preserve_last_n: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            stage1_enabled: true,
            stage2_enabled: true,
            stage3_enabled: true,
            collapse_threshold: 4,
            cluster_min_size: 3,
            preserve_last_n: 10,
        }
    }
}

/// Compaction statistics
#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    pub original_count: usize,
    pub final_count: usize,
    pub stage1_removed: usize,
    pub stage2_collapsed: usize,
    pub stage3_clustered: usize,
    pub tokens_saved_estimate: usize,
}

impl CompactionStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.final_count == 0 {
            1.0
        } else {
            self.original_count as f64 / self.final_count as f64
        }
    }

    pub fn report(&self) -> String {
        format!(
            "Session Compaction Complete\n\
             ├─ Original: {} messages\n\
             ├─ Final: {} messages ({:.1}x compression)\n\
             ├─ Stage 1 (Supersede): {} removed\n\
             ├─ Stage 2 (Collapse): {} collapsed\n\
             ├─ Stage 3 (Cluster): {} clustered\n\
             └─ Est. tokens saved: ~{}",
            self.original_count,
            self.final_count,
            self.compression_ratio(),
            self.stage1_removed,
            self.stage2_collapsed,
            self.stage3_clustered,
            self.tokens_saved_estimate
        )
    }
}

/// Message representation for compaction
#[derive(Debug, Clone)]
pub struct CompactMessage {
    pub id: usize,
    pub role: String,
    pub summary: String,
    pub tool_calls: Vec<String>,
    pub file_ops: Vec<FileOperation>,
}

#[derive(Debug, Clone)]
pub struct FileOperation {
    pub op_type: FileOpType,
    pub path: String,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileOpType {
    Read,
    Write,
    Edit,
}

/// Stage 1: Remove obsolete file operations
fn stage1_supersede(messages: Vec<CompactMessage>) -> (Vec<CompactMessage>, usize) {
    let mut file_ops: BTreeMap<String, Vec<FileOperation>> = BTreeMap::new();

    // Index file operations by path
    for (idx, msg) in messages.iter().enumerate() {
        for op in &msg.file_ops {
            file_ops
                .entry(op.path.clone())
                .or_default()
                .push(FileOperation {
                    op_type: op.op_type.clone(),
                    path: op.path.clone(),
                    index: idx,
                });
        }
    }

    let mut obsolete_indices: BTreeSet<usize> = BTreeSet::new();

    for ops in file_ops.values() {
        if ops.len() < 2 {
            continue;
        }

        // Find last write
        let last_write_idx = ops
            .iter()
            .filter(|op| matches!(op.op_type, FileOpType::Write | FileOpType::Edit))
            .map(|op| op.index)
            .max();

        if let Some(last_write) = last_write_idx {
            for op in ops {
                if op.index < last_write {
                    obsolete_indices.insert(op.index);
                }
            }
        }
    }

    let removed = obsolete_indices.len();
    let filtered: Vec<_> = messages
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !obsolete_indices.contains(i))
        .map(|(_, msg)| msg)
        .collect();

    (filtered, removed)
}

/// Stage 2: Collapse chatty message chains
fn stage2_collapse(
    messages: Vec<CompactMessage>,
    threshold: usize,
) -> (Vec<CompactMessage>, usize) {
    let mut result = Vec::new();
    let mut collapsed = 0;
    let mut i = 0;

    while i < messages.len() {
        let msg = &messages[i];

        // Check for chatty pattern (short back-and-forth)
        if msg.summary.len() < 20 && msg.tool_calls.is_empty() {
            let mut chain = vec![msg.clone()];
            let mut j = i + 1;

            while j < messages.len() && chain.len() < threshold {
                let next = &messages[j];
                if next.summary.len() < 20 && next.tool_calls.is_empty() {
                    chain.push(next.clone());
                    j += 1;
                } else {
                    break;
                }
            }

            if chain.len() >= 2 {
                collapsed += chain.len();
                result.push(CompactMessage {
                    id: chain.first().unwrap().id,
                    role: "assistant".to_string(),
                    summary: format!("[{} chatty messages collapsed]", chain.len()),
                    tool_calls: vec![],
                    file_ops: vec![],
                });
                i = j;
                continue;
            }
        }

        result.push(msg.clone());
        i += 1;
    }

    (result, collapsed)
}

/// Stage 3: Cluster similar messages
fn stage3_cluster(messages: Vec<CompactMessage>, min_size: usize) -> (Vec<CompactMessage>, usize) {
    // Simple clustering by keyword overlap
    let mut clusters: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (idx, msg) in messages.iter().enumerate() {
        // Extract keywords from summary
        let keywords: Vec<_> = msg
            .summary
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .take(3)
            .collect();

        let key = keywords.join("_");
        if !key.is_empty() {
            clusters.entry(key).or_default().push(idx);
        }
    }

    // Merge clusters that meet minimum size
    let mut clustered_count = 0;
    let mut result = messages.clone();

    for (key, indices) in &clusters {
        if indices.len() >= min_size {
            clustered_count += indices.len() - 1; // Keep first, merge rest

            result[indices[0]].summary = format!(
                "[Cluster: {} messages about '{}']",
                indices.len(),
                key.replace('_', " ")
            );

            for &idx in &indices[1..] {
                result[idx].summary = "[MERGED]".to_string();
            }
        }
    }

    // Remove merged messages
    result.retain(|msg| msg.summary != "[MERGED]");

    (result, clustered_count)
}

/// Main compaction entry point
pub fn compact_session(
    messages: Vec<CompactMessage>,
    config: CompactionConfig,
) -> (Vec<CompactMessage>, CompactionStats) {
    let original_count = messages.len();
    let mut stats = CompactionStats {
        original_count,
        ..Default::default()
    };

    let mut current = messages;

    // Stage 1: Supersede
    if config.stage1_enabled {
        let (filtered, removed);
        (filtered, removed) = stage1_supersede(current);
        current = filtered;
        stats.stage1_removed = removed;
    }

    // Stage 2: Collapse
    if config.stage2_enabled {
        let (collapsed, count);
        (collapsed, count) = stage2_collapse(current, config.collapse_threshold);
        current = collapsed;
        stats.stage2_collapsed = count;
    }

    // Stage 3: Cluster
    if config.stage3_enabled {
        let (clustered, count);
        (clustered, count) = stage3_cluster(current, config.cluster_min_size);
        current = clustered;
        stats.stage3_clustered = count;
    }

    stats.final_count = current.len();
    stats.tokens_saved_estimate = (stats.original_count.saturating_sub(stats.final_count)) * 100;

    (current, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_stats_ratio() {
        let stats = CompactionStats {
            original_count: 100,
            final_count: 25,
            ..Default::default()
        };
        assert!((stats.compression_ratio() - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_stage1_supersede_removes_obsolete() {
        let messages = vec![
            CompactMessage {
                id: 0,
                role: "user".to_string(),
                summary: "read file".to_string(),
                tool_calls: vec![],
                file_ops: vec![FileOperation {
                    op_type: FileOpType::Read,
                    path: "test.rs".to_string(),
                    index: 0,
                }],
            },
            CompactMessage {
                id: 1,
                role: "assistant".to_string(),
                summary: "write file".to_string(),
                tool_calls: vec![],
                file_ops: vec![FileOperation {
                    op_type: FileOpType::Write,
                    path: "test.rs".to_string(),
                    index: 1,
                }],
            },
        ];

        let (result, removed) = stage1_supersede(messages);
        assert_eq!(removed, 1); // First read should be removed
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_default_config() {
        let config = CompactionConfig::default();
        assert!(config.stage1_enabled);
        assert!(config.stage2_enabled);
        assert!(config.stage3_enabled);
        assert_eq!(config.collapse_threshold, 4);
        assert_eq!(config.cluster_min_size, 3);
    }
}
