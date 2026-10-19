## WITH NO SIMD (YET): HAS OPEN ADDRESSED HASHTABLE
for i in 1 2 3 4 5; do time ./target/release/one_brc; done
./target/release/one_brc  59.82s user 12.03s system 881% cpu 8.150 total
./target/release/one_brc  59.33s user 25.05s system 831% cpu 10.145 total
./target/release/one_brc  62.68s user 3.22s system 1161% cpu 5.674 total
./target/release/one_brc  63.08s user 1.50s system 1416% cpu 4.561 total
./target/release/one_brc  64.26s user 2.96s system 1271% cpu 5.287 total


./target/release/one_brc  34.46s user 26.67s system 832% cpu 7.345 total
./target/release/one_brc  35.93s user 9.85s system 849% cpu 5.388 total
./target/release/one_brc  37.26s user 8.86s system 907% cpu 5.080 total
./target/release/one_brc  37.23s user 5.93s system 995% cpu 4.336 total
./target/release/one_brc  37.37s user 8.78s system 933% cpu 4.943 total
