//! Compressed Sparse Row (CSR) graph storage and Tarjan's SCC algorithm.
//!
//! Code generation builds this graph once and traverses successors repeatedly
//! during one `O(|V| + |E|)` depth-first pass. Sorted, deduplicated edge input
//! makes traversal and component assignment deterministic while CSR keeps the
//! temporary representation to two contiguous allocations.

/// A compact directed graph stored in compressed sparse row format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
  /// Slice offsets into `successors`; length is the vertex count plus one.
  offsets: Vec<usize>,
  /// Contiguous destination indices for every directed edge.
  successors: Vec<usize>,
}

impl Graph {
  /// Builds a graph from directed `[source, destination]` pairs.
  ///
  /// Edge pairs are sorted and deduplicated for deterministic traversal.
  pub fn from_edges(mut edges: Vec<[usize; 2]>, vertex_count: usize) -> Self {
    edges.sort_unstable();
    edges.dedup();

    // Count each source's edges in the slot after its eventual start offset.
    let mut offsets = vec![0; vertex_count + 1];
    for &[source, _] in &edges {
      offsets[source + 1] += 1;
    }

    // A prefix sum converts those degrees to CSR slice boundaries.
    for vertex in 0..vertex_count {
      offsets[vertex + 1] += offsets[vertex];
    }

    // Sorted pairs already place each source's destinations contiguously.
    let successors = edges.into_iter().map(|[_, destination]| destination).collect();
    Self { offsets, successors }
  }

  /// Computes strongly connected components in reverse topological order.
  ///
  /// Returns a vector mapping each vertex index to its assigned SCC component ID.
  pub fn strongly_connected_components(&self) -> Vec<usize> {
    Tarjan::new(self).run()
  }

  /// Returns the number of vertices in the graph.
  pub fn len(&self) -> usize {
    self.offsets.len().saturating_sub(1)
  }

  /// Returns the outgoing successors of `vertex`.
  pub fn successors(&self, vertex: usize) -> &[usize] {
    match self.offsets.get(vertex..) {
      Some(&[start, end, ..]) if let Some(slice) = self.successors.get(start..end) => slice,
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
  components: Vec<usize>,
  /// Total number of strongly connected components identified so far.
  component_count: usize,
  /// Next 1-based DFS discovery index.
  next_index: usize,
}

impl<'a> Tarjan<'a> {
  /// Sentinel for unvisited vertices.
  const UNVISITED: usize = 0;
  /// Sentinel for vertices assigned to an SCC and popped from the stack.
  const ASSIGNED: usize = !0;

  /// Initializes Tarjan algorithm state for `graph`.
  fn new(graph: &'a Graph) -> Self {
    let vertex_count = graph.len();
    let states = vec![Self::UNVISITED; vertex_count];
    let stack = Vec::with_capacity(vertex_count);
    let components = vec![0; vertex_count];
    Self { graph, states, stack, components, component_count: 0, next_index: 1 }
  }

  /// Traverses one vertex recursively and returns its low-link value.
  fn strong_connect(&mut self, vertex: usize) -> usize {
    let (vertex_index, stack_depth) = (self.next_index, self.stack.len());
    self.next_index += 1;

    // Discover vertex: record its 1-based DFS index and push onto active stack.
    self.states[vertex] = vertex_index;
    self.stack.push(vertex);

    // Compute the low-link value across outgoing edges.
    let lowlink = self.graph.successors(vertex).iter().fold(vertex_index, |lowlink, &succ| match self.states[succ] {
      // An unvisited successor extends the DFS tree.
      Self::UNVISITED => lowlink.min(self.strong_connect(succ)),
      // A completed successor cannot belong to the active component.
      Self::ASSIGNED => lowlink,
      // An active successor contributes its discovery index.
      successor_index => lowlink.min(successor_index),
    });

    // A vertex whose low-link equals its discovery index roots one component.
    if lowlink == vertex_index {
      for member in self.stack.drain(stack_depth..) {
        self.states[member] = Self::ASSIGNED;
        self.components[member] = self.component_count;
      }
      self.component_count += 1;
    }

    lowlink
  }

  /// Traverses all unvisited vertices and returns the resulting SCC mapping.
  fn run(mut self) -> Vec<usize> {
    for vertex in 0..self.graph.len() {
      if let Self::UNVISITED = self.states[vertex] {
        self.strong_connect(vertex);
      }
    }

    self.components
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
    let scc = graph.strongly_connected_components();
    assert_matches!(&*scc, []);
  }

  #[test]
  fn disconnected_nodes() {
    let graph = Graph::from_edges(vec![], 3);
    assert_eq!(graph.len(), 3);
    // Each isolated vertex is its own separate component.
    let scc = graph.strongly_connected_components();
    assert_matches!(&*scc, [0, 1, 2]);
  }

  #[test]
  fn simple_cycle() {
    // 0 -> 1 -> 2 -> 0
    let edges = vec![[0, 1], [1, 2], [2, 0]];
    let graph = Graph::from_edges(edges, 3);
    // All 3 vertices form a single SCC.
    let scc = graph.strongly_connected_components();
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
    let scc = graph.strongly_connected_components();
    assert_matches!(&*scc, [a0, a1, b0, b1] if a0 == a1 && b0 == b1 && a0 != b0 && b0 < a0);
  }

  #[test]
  fn self_loops() {
    // 0 -> 0 (self-loop), 1 -> 2
    let edges = vec![[0, 0], [1, 2]];
    let graph = Graph::from_edges(edges, 3);
    let scc = graph.strongly_connected_components();
    assert_matches!(&*scc, [a, b, c] if a != b && b != c && a != c);
  }
}
