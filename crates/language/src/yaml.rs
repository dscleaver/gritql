use crate::language::{
    fields_for_nodes, normalize_double_quote_string, normalize_identity, Field,
    LeafEquivalenceClass, LeafNormalizer, MarzanoLanguage, NodeTypes, SortId, TSLanguage,
};
use grit_util::Language;
use marzano_util::node_with_source::NodeWithSource;
use std::{sync::OnceLock, vec};

static NODE_TYPES_STRING: &str = include_str!("../../../resources/node-types/yaml-node-types.json");
static NODE_TYPES: OnceLock<Vec<Vec<Field>>> = OnceLock::new();
static LANGUAGE: OnceLock<TSLanguage> = OnceLock::new();
static EQUIVALENT_LEAF_NODES: OnceLock<Vec<Vec<LeafNormalizer>>> = OnceLock::new();

#[cfg(not(feature = "builtin-parser"))]
fn built_in_language() -> TSLanguage {
    unimplemented!(
        "tree-sitter parser must be initialized before use when [builtin-parser] is off."
    )
}
#[cfg(feature = "builtin-parser")]
fn built_in_language() -> TSLanguage {
    tree_sitter_yaml::language().into()
}

#[derive(Debug, Clone)]
pub struct Yaml {
    node_types: &'static [Vec<Field>],
    metavariable_sort: SortId,
    comment_sort: SortId,
    equivalent_leaf_nodes: &'static [Vec<LeafNormalizer>],
    language: &'static TSLanguage,
}

impl Yaml {
    pub(crate) fn new(lang: Option<TSLanguage>) -> Self {
        let language = LANGUAGE.get_or_init(|| lang.unwrap_or_else(built_in_language));
        let node_types = NODE_TYPES.get_or_init(|| fields_for_nodes(language, NODE_TYPES_STRING));
        let equivalent_leaf_nodes = EQUIVALENT_LEAF_NODES.get_or_init(|| {
            vec![vec![
                LeafNormalizer::new(
                    language.id_for_node_kind("string_scalar", true),
                    normalize_identity,
                ),
                LeafNormalizer::new(
                    language.id_for_node_kind("double_quote_scalar", true),
                    normalize_double_quote_string,
                ),
            ]]
        });
        let metavariable_sort = language.id_for_node_kind("grit_metavariable", true);
        let comment_sort = language.id_for_node_kind("comment", true);
        Self {
            node_types,
            metavariable_sort,
            comment_sort,
            equivalent_leaf_nodes,
            language,
        }
    }
    pub(crate) fn is_initialized() -> bool {
        LANGUAGE.get().is_some()
    }
}

impl NodeTypes for Yaml {
    fn node_types(&self) -> &[Vec<Field>] {
        self.node_types
    }
}

impl Language for Yaml {
    use_marzano_delegate!();

    fn language_name(&self) -> &'static str {
        "YAML"
    }

    fn snippet_context_strings(&self) -> &[(&'static str, &'static str)] {
        &[("", "")]
    }

    fn comment_prefix(&self) -> &'static str {
        "#"
    }

    // Given a character, return the character that should be used to pad the snippet (if any)
    fn take_padding(&self, current: char, next: Option<char>) -> Option<char> {
        if current.is_whitespace() {
            Some(current)
        } else if current == '-' && next == Some(' ') {
            Some(' ')
        } else {
            None
        }
    }

    fn should_pad_snippet(&self) -> bool {
        true
    }

    fn make_single_line_comment(&self, text: &str) -> String {
        format!("# {}\n", text)
    }
}

impl<'a> MarzanoLanguage<'a> for Yaml {
    fn get_ts_language(&self) -> &TSLanguage {
        self.language
    }

    fn is_comment_sort(&self, id: SortId) -> bool {
        id == self.comment_sort
    }

    fn metavariable_sort(&self) -> SortId {
        self.metavariable_sort
    }

    fn get_equivalence_class(
        &self,
        sort: SortId,
        text: &str,
    ) -> Result<Option<LeafEquivalenceClass>, String> {
        if let Some(class) = self
            .equivalent_leaf_nodes
            .iter()
            .find(|v| v.iter().any(|n| n.sort() == sort))
        {
            LeafEquivalenceClass::new(text, sort, class)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {

    use marzano_util::print_node::print_node;

    use crate::language::nodes_from_indices;

    use super::*;

    #[test]
    fn simple_yaml() {
        let snippet = "- foo:";
        let lang = Yaml::new(None);
        let snippets = lang.parse_snippet_contexts(snippet);
        let nodes = nodes_from_indices(&snippets);
        for node in &nodes {
            print_node(&node.node)
        }
        assert!(!nodes.is_empty());
    }

    #[test]
    fn simple_yaml_metavariable() {
        let snippet = "$list";
        let lang = Yaml::new(None);
        let snippets = lang.parse_snippet_contexts(snippet);
        let nodes = nodes_from_indices(&snippets);
        let mut found_metavar = false;
        for node in &nodes {
            print_node(&node.node);
            if node.node.kind_id() == lang.metavariable_sort() {
                found_metavar = true;
            }
        }
        assert!(!nodes.is_empty());
        assert!(found_metavar);
    }
}
