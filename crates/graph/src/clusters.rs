//! Subtemas: grupos pequenos de itens ligados entre si, dentro de cada tema.
//! É o que faz as ligações viajarem juntas em Feixes.

use crate::{Node, SLOTS};

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

pub fn assign_subs(nodes: &mut [Node], links: &[(u32, u32)]) {
    let n = nodes.len();
    let mut count = [0usize; SLOTS];
    for x in nodes.iter() {
        count[x.theme as usize] += 1;
    }
    let cap: Vec<usize> = count.iter().map(|&c| ((c as f32).sqrt() * 2.4).round().max(3.0) as usize).collect();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    let mut size = vec![1usize; n];
    for &(a, b) in links {
        if nodes[a as usize].theme != nodes[b as usize].theme {
            continue;
        }
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        if ra != rb && size[ra as usize] + size[rb as usize] <= cap[nodes[a as usize].theme as usize] {
            parent[rb as usize] = ra;
            size[ra as usize] += size[rb as usize];
        }
    }
    // Sozinhos dividem um balde: subtema de um item só não forma feixe.
    let mut open: [(u32, usize); SLOTS] = [(u32::MAX, 0); SLOTS];
    let mut label = vec![u32::MAX; n];
    let mut next = 0u32;
    for (i, node) in nodes.iter_mut().enumerate() {
        let root = find(&mut parent, i as u32) as usize;
        if size[root] > 1 {
            if label[root] == u32::MAX {
                label[root] = next;
                next += 1;
            }
            node.sub = label[root];
            continue;
        }
        let t = node.theme as usize;
        if open[t].0 == u32::MAX || open[t].1 >= cap[t] {
            open[t] = (next, 0);
            next += 1;
        }
        open[t].1 += 1;
        node.sub = open[t].0;
    }
}
