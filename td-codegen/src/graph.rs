//! Compressed Sparse Row (CSR) graph representation and Tarjan's SCC algorithm.
//!
//! Identifies strongly connected components in `O(|V| + |E|)` time via a single
//! depth-first search pass.

/// A compact directed graph stored in Compressed Sparse Row (CSR) format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrGraph {
  /// Slice offsets into `edges` for each node's outgoing neighbors; length is `nodes + 1`.
  offsets: Vec<usize>,
  /// Destination node indices for all directed edges.
  edges: Vec<usize>,
}

impl CsrGraph {
  /// Constructs a graph from `[source, destination]` edge pairs and total node count.
  ///
  /// Deduplicates and sorts edges for deterministic traversal.
  pub fn from_pairs(mut edges: Vec<[usize; 2]>, nodes: usize) -> Self {
    edges.sort_unstable();
    edges.dedup();

    // Compute out-degrees for each source node.
    let mut offsets = vec![0; nodes + 1];
    for &[src, _] in &edges {
      offsets[src + 1] += 1;
    }

    // Convert degrees to slice offsets via prefix sum.
    for i in 0..nodes {
      offsets[i + 1] += offsets[i];
    }

    // Pack destination nodes into a contiguous array.
    let edges = edges.into_iter().map(|[_, dst]| dst).collect();
    Self { offsets, edges }
  }

  /// Computes strongly connected components in reverse topological order.
  ///
  /// Returns a vector mapping each node index to its assigned SCC group ID.
  pub fn scc(&self) -> Vec<usize> {
    Tarjan::new(self).run()
  }

  /// Returns the number of nodes in the graph.
  pub fn len(&self) -> usize {
    self.offsets.len().saturating_sub(1)
  }

  /// Returns the outgoing neighbors for `node`.
  pub fn neighbors(&self, node: usize) -> &[usize] {
    match self.offsets.get(node..) {
      Some(&[start, end, ..]) if let Some(edges) = self.edges.get(start..end) => edges,
      _ => &[],
    }
  }
}

/// Execution state for Tarjan's strongly connected components algorithm.
struct Tarjan<'a> {
  /// Underlying directed graph.
  graph: &'a CsrGraph,
  /// Unified node state:
  /// - `UNVISITED` (`0`): unvisited node.
  /// - `ASSIGNED` (`!0`): committed to an SCC and popped from the stack.
  /// - `1..usize::MAX`: 1-based DFS discovery index while on the active stack.
  states: Vec<usize>,
  /// Active traversal stack of nodes in the current DFS path.
  stack: Vec<usize>,
  /// SCC group ID assigned to each node.
  scc: Vec<usize>,
  /// Total number of strongly connected components identified so far.
  scc_count: usize,
  /// Next 1-based DFS discovery index.
  index: usize,
}

impl<'a> Tarjan<'a> {
  /// Sentinel for unvisited nodes.
  const UNVISITED: usize = 0;
  /// Sentinel for nodes assigned to an SCC and popped from the stack.
  const ASSIGNED: usize = !0;

  /// Initializes Tarjan algorithm state for `graph`.
  fn new(graph: &'a CsrGraph) -> Self {
    let nodes = graph.len();
    let states = vec![Self::UNVISITED; nodes];
    let stack = Vec::with_capacity(nodes);
    let scc = vec![0; nodes];
    Self { graph, states, stack, scc, scc_count: 0, index: 1 }
  }

  /// Traverses `node` recursively (`strongconnect`), returning its low-link value.
  fn visit(&mut self, node: usize) -> usize {
    let (node_idx, depth) = (self.index, self.stack.len());
    self.index += 1;

    // Discover node: record its 1-based DFS index and push onto active stack.
    self.states[node] = node_idx;
    self.stack.push(node);

    // Compute low-link value across outgoing edges.
    let low = self.graph.neighbors(node).iter().fold(node_idx, |low, &n| match self.states[n] {
      // Neighbor is unvisited; recurse into it.
      Self::UNVISITED => low.min(self.visit(n)),
      // Neighbor belongs to a completed SCC; ignore it.
      Self::ASSIGNED => low,
      // Neighbor is on the stack in the active path.
      n_idx => low.min(n_idx),
    });

    // If low-link equals discovery index, `node` is the root of an SCC.
    if low == node_idx {
      for top in self.stack.drain(depth..) {
        self.states[top] = Self::ASSIGNED;
        self.scc[top] = self.scc_count;
      }
      self.scc_count += 1;
    }

    low
  }

  /// Traverses all unvisited nodes and returns the resulting SCC mapping.
  fn run(mut self) -> Vec<usize> {
    for node in 0..self.graph.len() {
      if self.states[node] == Self::UNVISITED {
        self.visit(node);
      }
    }

    self.scc
  }
}

#[cfg(test)]
mod tests {
  use std::assert_matches;

  use super::*;

  #[test]
  fn empty_graph() {
    let graph = CsrGraph::from_pairs(vec![], 0);
    assert_eq!(graph.len(), 0);
    assert_matches!(&*graph.scc(), []);
  }

  #[test]
  fn disconnected_nodes() {
    let graph = CsrGraph::from_pairs(vec![], 3);
    assert_eq!(graph.len(), 3);
    // Each isolated node is its own separate component.
    assert_matches!(&*graph.scc(), [0, 1, 2]);
  }

  #[test]
  fn simple_cycle() {
    // 0 -> 1 -> 2 -> 0
    let edges = vec![[0, 1], [1, 2], [2, 0]];
    let graph = CsrGraph::from_pairs(edges, 3);
    // All 3 nodes form a single SCC.
    assert_matches!(&*graph.scc(), [a, b, c] if a == b && b == c);
  }

  #[test]
  fn two_components_with_cross_edge() {
    // Component A: 0 -> 1 -> 0
    // Component B: 2 -> 3 -> 2
    // Cross edge: 0 -> 2
    let edges = vec![[0, 1], [1, 0], [2, 3], [3, 2], [0, 2]];
    let graph = CsrGraph::from_pairs(edges, 4);
    // 0 and 1 are in component A, 2 and 3 are in component B, with B < A.
    assert_matches!(&*graph.scc(), [a0, a1, b0, b1] if a0 == a1 && b0 == b1 && a0 != b0 && b0 < a0);
  }

  #[test]
  fn self_loops() {
    // 0 -> 0 (self-loop), 1 -> 2
    let edges = vec![[0, 0], [1, 2]];
    let graph = CsrGraph::from_pairs(edges, 3);
    assert_matches!(&*graph.scc(), [a, b, c] if a != b && b != c && a != c);
  }
}
