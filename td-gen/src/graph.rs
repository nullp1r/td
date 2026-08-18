//! Graph data structures and algorithms.

/// A CSR (compressed sparse row) adjacency-list directed graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrGraph {
  offsets: Vec<usize>,
  edges: Vec<usize>,
}

impl CsrGraph {
  /// Constructs a CSR graph from unsorted `[src, dst]` edge pairs.
  pub fn from_pairs(mut edges: Vec<[usize; 2]>, nodes: usize) -> Self {
    edges.sort_unstable();
    edges.dedup();

    let mut offsets = vec![0; nodes + 1];
    for &[src, _] in &edges {
      offsets[src + 1] += 1;
    }
    for i in 0..nodes {
      offsets[i + 1] += offsets[i];
    }

    let edges = edges.into_iter().map(|[_, dst]| dst).collect();
    Self { offsets, edges }
  }

  /// Computes the strongly connected components (SCC) using Tarjan's algorithm.
  ///
  /// Returns a vector mapping each node to its component ID.
  pub fn scc(&self) -> Vec<usize> {
    Tarjan::new(self).run()
  }

  /// Returns the number of nodes in the graph.
  fn len(&self) -> usize {
    self.offsets.len().saturating_sub(1)
  }

  /// Returns the outgoing neighbors of `node`.
  fn neighbors(&self, node: usize) -> &[usize] {
    match self.offsets.get(node..) {
      Some(&[i, j, ..]) if let Some(edges) = self.edges.get(i..j) => edges,
      _ => &[],
    }
  }
}

struct Tarjan<'a> {
  graph: &'a CsrGraph,
  states: Vec<usize>,
  stack: Vec<usize>,
  scc: Vec<usize>,
  idx: usize,
  count: usize,
}

impl<'a> Tarjan<'a> {
  const UNVISITED: usize = 0;
  const VISITED: usize = !0;

  fn new(graph: &'a CsrGraph) -> Self {
    let nodes = graph.len();
    let states = vec![Self::UNVISITED; nodes];
    let stack = Vec::with_capacity(nodes);
    let scc = vec![0; nodes];
    Self { graph, states, stack, scc, idx: 1, count: 0 }
  }

  fn visit(&mut self, node: usize) -> usize {
    let (idx, depth) = (self.idx, self.stack.len());
    self.idx += 1;
    self.states[node] = idx;
    self.stack.push(node);

    let low = self.graph.neighbors(node).iter().fold(idx, |low, &n| match self.states[n] {
      Self::UNVISITED => low.min(self.visit(n)),
      Self::VISITED => low,
      idx => low.min(idx),
    });

    if low == idx {
      for top in self.stack.drain(depth..) {
        self.states[top] = Self::VISITED;
        self.scc[top] = self.count;
      }
      self.count += 1;
    }

    low
  }

  fn run(mut self) -> Vec<usize> {
    for node in 0..self.graph.len() {
      if self.states[node] == Self::UNVISITED {
        self.visit(node);
      }
    }

    self.scc
  }
}
