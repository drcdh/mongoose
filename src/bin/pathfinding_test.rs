use array2d::Array2D;
use bevy::utils::petgraph::{algo::dijkstra, graph::NodeIndex, Graph, Undirected};

struct Arena(Array2D<bool>);

fn main() {
    let rows = vec![
        vec![true, false, true, true, false],
        vec![true, false, true, false, false],
        vec![false, false, false, false, true],
        vec![false, true, false, true, true],
        vec![false, true, true, true, true],
    ];
    let arena = Arena(Array2D::from_rows(&rows).unwrap());

    let mut graph = Graph::<(), (), Undirected>::new_undirected();
    let nodes = Array2D::<NodeIndex>::filled_by_column_major(|| graph.add_node(()), 5, 5);
    for i in 0..5 as usize {
        for j in 0..5 as usize {
            print!("{:?}", nodes[(i, j)]);
            if arena.0[(i, j)] {
                continue;
            }
            if i < 4 && !arena.0[(i + 1, j)] {
                graph.add_edge(nodes[(i, j)], nodes[(i + 1, j)], ());
            }
            if j < 4 && !arena.0[(i, j + 1)] {
                graph.add_edge(nodes[(i, j)], nodes[(i, j + 1)], ());
            }
        }
        println!();
    }
    let path = dijkstra(&graph, nodes[(4, 0)], Some(nodes[(0, 4)]), |_| 1);
    println!("{:?}", path);
}
