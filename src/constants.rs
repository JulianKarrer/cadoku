use const_for::const_for;

const CELLS:[[usize; 9];9] = [
    [0 , 1 , 2 , 9 , 10, 11, 18, 19, 20,],
    [3 , 4 , 5 , 12, 13, 14, 21, 22, 23,],
    [6 , 7 , 8 , 15, 16, 17, 24, 25, 26,],
    [27, 28, 29, 36, 37, 38, 45, 46, 47,],
    [30, 31, 32, 39, 40, 41, 48, 49, 50,],
    [33, 34, 35, 42, 43, 44, 51, 52, 53,],
    [54, 55, 56, 63, 64, 65, 72, 73, 74,],
    [57, 58, 59, 66, 67, 68, 75, 76, 77,],
    [60, 61, 62, 69, 70, 71, 78, 79, 80 ],
];

pub const UNITS : [[[usize; 9]; 3]; 81] = GENERATE_UNITS();
pub const PEERS:[[usize; 20]; 81] = GENERATE_PEERS();


const fn GENERATE_UNITS()->[[[usize; 9]; 3]; 81]{
    let mut res:[[[usize; 9]; 3]; 81] = [[[0; 9]; 3]; 81];
    // for each square
    const_for!(s in 0..81 => {
        // add row
        let row_0 = (s / 9)*9;
        let mut i = 0;
        const_for!(rn in row_0..row_0+9 => {
            res[s][0][i] = rn;
            i += 1;
        });
        i = 0;
        // add column
        let col_0 = s % 9;
        const_for!(cn in (col_0..col_0+72+1).step_by(9) => {
            res[s][1][i] = cn;
            i += 1;
        });
        // add cell
        const_for!(c in 0..9 => {
            // check if cell c contains the square s
            let mut contains = false;
            const_for!(ci in 0..9 => {
                if CELLS[c][ci] == s{contains = true;}
            });
            // if so, copy the cell
            if contains {
                const_for!(i in 0..9 => {
                    res[s][2][i] = CELLS[c][i];
                });
            }
        });
    });
    res
}

const fn GENERATE_PEERS()->[[usize; 20]; 81]{
    // buffer to hold results
    let mut res:[[usize; 20]; 81] = [[0;20];81];
    // buffer to hold the highest index of each resulting list
    // -> poor man's Vec<_>
    let mut i: [usize; 81] = [0;81]; 

    const_for!(s in 0..81 => {
        // row neighbours
        let row_0 = (s / 9)*9;
        const_for!(rn in row_0..row_0+9 => {
            // add row neighbours that are not the current square to the peer list
            if rn != s{
                res[s][i[s]] = rn;
                i[s] += 1;
            }
        });
        // column neighbours
        let col_0 = s % 9;
        const_for!(cn in (col_0..col_0+72+1).step_by(9) => {
            // add row neighbours that are not the current square to the peer list
            if cn != s{
                res[s][i[s]] = cn;
                i[s] += 1;
            }
        });
        // cell neighbours
        const_for!(c in 0..9 => {
            // check if cell c contains the square s
            let mut contains = false;
            const_for!(ci in 0..9 => {
                if CELLS[c][ci] == s{
                    contains = true;
                }
            });
            // if so, add remaining peers
            if contains {
                // for each cell neighbour
                const_for!(cn in 0..9 => {
                    let cell_nbr = CELLS[c][cn];
                    // exclude if it is the square itself or already in res[0..i[s]]
                    let mut excluded = false;
                    const_for!(i_already_in in 0..i[s] => {
                        if cell_nbr == res[s][i_already_in] || cell_nbr == s{
                            excluded = true;
                        }
                    });
                    // otherwise add it
                    if !excluded{
                        res[s][i[s]] = cell_nbr;
                        i[s] += 1;
                    }
                });
            }
        });
    });
    res
}
