pub(super) struct Digraph {
    starts: Vec<usize>,
    targets: Vec<usize>,
}

const UNSEEN: usize = usize::MAX;

impl Digraph {
    pub(super) fn new(nodes: usize, edges: &[(usize, usize)]) -> Self {
        let mut starts = vec![0; nodes + 1];
        for &(from, _) in edges {
            starts[from + 1] += 1;
        }
        let mut total = 0;
        for start in &mut starts {
            total += *start;
            *start = total;
        }
        let mut next = starts.clone();
        let mut targets = vec![0; edges.len()];
        for &(from, to) in edges {
            targets[next[from]] = to;
            next[from] += 1;
        }
        Self { starts, targets }
    }

    fn nodes(&self) -> usize {
        self.starts.len() - 1
    }

    fn successors(&self, node: usize) -> &[usize] {
        &self.targets[self.starts[node]..self.starts[node + 1]]
    }

    pub(super) fn reversed(&self) -> Self {
        let edges: Vec<(usize, usize)> = (0..self.nodes())
            .flat_map(|from| self.successors(from).iter().map(move |&to| (to, from)))
            .collect();
        Self::new(self.nodes(), &edges)
    }

    /// Each node's strongly connected component, by Tarjan's algorithm (SIAM J. Comput. 1(2), 1972)
    /// with an explicit call stack, so a deep map cannot exhaust the thread's stack.
    pub(super) fn components(&self) -> Vec<usize> {
        let nodes = self.nodes();
        let mut order = vec![UNSEEN; nodes];
        let mut low = vec![0; nodes];
        let mut component = vec![UNSEEN; nodes];
        let mut stack = Vec::new();
        let mut calls: Vec<(usize, usize)> = Vec::new();
        let (mut discovered, mut components) = (0, 0);
        for root in 0..nodes {
            if order[root] != UNSEEN {
                continue;
            }
            let mut found = Some(root);
            loop {
                if let Some(node) = found.take() {
                    order[node] = discovered;
                    low[node] = discovered;
                    discovered += 1;
                    stack.push(node);
                    calls.push((node, self.starts[node]));
                }
                let Some(&(node, edge)) = calls.last() else {
                    break;
                };
                if edge < self.starts[node + 1] {
                    if let Some(call) = calls.last_mut() {
                        call.1 += 1;
                    }
                    let next = self.targets[edge];
                    if order[next] == UNSEEN {
                        found = Some(next);
                    } else if component[next] == UNSEEN {
                        low[node] = low[node].min(order[next]);
                    }
                    continue;
                }
                calls.pop();
                if let Some(&(caller, _)) = calls.last() {
                    low[caller] = low[caller].min(low[node]);
                }
                if low[node] == order[node] {
                    while let Some(member) = stack.pop() {
                        component[member] = components;
                        if member == node {
                            break;
                        }
                    }
                    components += 1;
                }
            }
        }
        component
    }

    pub(super) fn reachable(&self, seeds: &[usize]) -> Vec<bool> {
        let mut seen = vec![false; self.nodes()];
        for &seed in seeds {
            seen[seed] = true;
        }
        let mut todo = seeds.to_vec();
        while let Some(node) = todo.pop() {
            for &next in self.successors(node) {
                if !seen[next] {
                    seen[next] = true;
                    todo.push(next);
                }
            }
        }
        seen
    }
}
