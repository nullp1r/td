//! Compressed Sparse Row (CSR) graph storage and Tarjan's SCC algorithm.
//!
//! Code generation builds this graph once and traverses neighbors repeatedly
//! during one `O(|V| + |E|)` depth-first pass. Sorted, deduplicated edge input
//! makes traversal and component assignment deterministic while CSR keeps the
//! temporary representation to two contiguous allocations.

/// A compact directed graph stored in Compressed Sparse Row (CSR) format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
  /// Slice offsets into `edges`; length is the vertex count plus one.
  offsets: Vec<usize>,
  /// Contiguous destination indices for every directed edge.
  edges: Vec<usize>,
}

impl Graph {
  /// Constructs a graph from directed `[source, destination]` edge pairs and total vertex count.
  ///
  /// Edge pairs are sorted and deduplicated for deterministic traversal.
  pub fn from_edges(mut edges: Vec<[usize; 2]>, vertices: usize) -> Self {
    edges.sort_unstable();
    edges.dedup();

    // Count each source's edges in the slot after its eventual start offset.
    let mut offsets = vec![0; vertices + 1];
    for &[u, _] in &edges {
      offsets[u + 1] += 1;
    }

    // A prefix sum converts those degrees to CSR slice boundaries.
    for i in 0..vertices {
      offsets[i + 1] += offsets[i];
    }

    // Sorted pairs already place each source's destinations contiguously.
    let edges = edges.into_iter().map(|[_, v]| v).collect();
    Self { offsets, edges }
  }

  /// Computes strongly connected components in reverse topological order.
  ///
  /// Returns a vector mapping each vertex index to its assigned SCC component ID.
  pub fn scc(&self) -> Vec<usize> {
    Tarjan::new(self).run()
  }

  /// Returns the number of vertices in the graph.
  pub fn len(&self) -> usize {
    self.offsets.len().saturating_sub(1)
  }

  /// Returns the outgoing neighbor vertices for vertex `v`.
  pub fn neighbors(&self, v: usize) -> &[usize] {
    match self.offsets.get(v..) {
      Some(&[start, end, ..]) if let Some(ns) = self.edges.get(start..end) => ns,
      _ => &[],
    }
  }
}

/// Execution state for Tarjan's strongly connected components algorithm.
struct Tarjan<'a> {
  /// Underlying directed graph.
  graph: &'a Graph,
  /// Unified vertex state:
  /// - `UNVISITED` (`0`): unvisited vertex.
  /// - `ASSIGNED` (`!0`): committed to an SCC and popped from the stack.
  /// - `1..usize::MAX`: 1-based DFS discovery index while on the active stack.
  states: Vec<usize>,
  /// Active traversal stack of vertices in the current DFS path.
  stack: Vec<usize>,
  /// SCC component ID assigned to each vertex.
  scc: Vec<usize>,
  /// Total number of strongly connected components identified so far.
  scc_count: usize,
  /// Next 1-based DFS discovery index.
  index: usize,
}

impl<'a> Tarjan<'a> {
  /// Sentinel for unvisited vertices.
  const UNVISITED: usize = 0;
  /// Sentinel for vertices assigned to an SCC and popped from the stack.
  const ASSIGNED: usize = !0;

  /// Initializes Tarjan algorithm state for `graph`.
  fn new(graph: &'a Graph) -> Self {
    let vertices = graph.len();
    let states = vec![Self::UNVISITED; vertices];
    let stack = Vec::with_capacity(vertices);
    let scc = vec![0; vertices];
    Self { graph, states, stack, scc, scc_count: 0, index: 1 }
  }

  /// Traverses vertex `v` recursively, returning its low-link value.
  fn strong_connect(&mut self, v: usize) -> usize {
    let (v_index, depth) = (self.index, self.stack.len());
    self.index += 1;

    // Discover vertex: record its 1-based DFS index and push onto active stack.
    self.states[v] = v_index;
    self.stack.push(v);

    // Compute low-link value across outgoing edges (v, w).
    let lowlink = self.graph.neighbors(v).iter().fold(v_index, |low, &w| match self.states[w] {
      // Successor w is unvisited; recurse into it.
      Self::UNVISITED => low.min(self.strong_connect(w)),
      // Successor w belongs to a completed SCC; ignore it.
      Self::ASSIGNED => low,
      // Successor w is on the stack in the current DFS path.
      w_index => low.min(w_index),
    });

    // If low-link equals discovery index, vertex `v` is the root of an SCC.
    if lowlink == v_index {
      for w in self.stack.drain(depth..) {
        self.states[w] = Self::ASSIGNED;
        self.scc[w] = self.scc_count;
      }
      self.scc_count += 1;
    }

    lowlink
  }

  /// Traverses all unvisited vertices and returns the resulting SCC mapping.
  fn run(mut self) -> Vec<usize> {
    for v in 0..self.graph.len() {
      if let Self::UNVISITED = self.states[v] {
        self.strong_connect(v);
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
    let graph = Graph::from_edges(vec![], 0);
    assert_eq!(graph.len(), 0);
    let scc = graph.scc();
    assert_matches!(&*scc, []);
  }

  #[test]
  fn disconnected_nodes() {
    let graph = Graph::from_edges(vec![], 3);
    assert_eq!(graph.len(), 3);
    // Each isolated vertex is its own separate component.
    let scc = graph.scc();
    assert_matches!(&*scc, [0, 1, 2]);
  }

  #[test]
  fn simple_cycle() {
    // 0 -> 1 -> 2 -> 0
    let edges = vec![[0, 1], [1, 2], [2, 0]];
    let graph = Graph::from_edges(edges, 3);
    // All 3 vertices form a single SCC.
    let scc = graph.scc();
    assert_matches!(&*scc, [a, b, c] if a == b && b == c);
  }

  #[test]
  fn two_components_with_cross_edge() {
    // Component A: 0 -> 1 -> 0
    // Component B: 2 -> 3 -> 2
    // Cross edge: 0 -> 2
    let edges = vec![[0, 1], [1, 0], [2, 3], [3, 2], [0, 2]];
    let graph = Graph::from_edges(edges, 4);
    // 0 and 1 are in component A, 2 and 3 are in component B, with B < A.
    let scc = graph.scc();
    assert_matches!(&*scc, [a0, a1, b0, b1] if a0 == a1 && b0 == b1 && a0 != b0 && b0 < a0);
  }

  #[test]
  fn self_loops() {
    // 0 -> 0 (self-loop), 1 -> 2
    let edges = vec![[0, 0], [1, 2]];
    let graph = Graph::from_edges(edges, 3);
    let scc = graph.scc();
    assert_matches!(&*scc, [a, b, c] if a != b && b != c && a != c);
  }
}
