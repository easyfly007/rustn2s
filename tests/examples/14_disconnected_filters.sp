* Two Disconnected RC Filters
* Stress-test for the placer's DAG handling: this netlist has no edges
* between the two filter sub-graphs (they share only ground). A correct
* layout draws them side-by-side or stacked, but with consistent
* internal flow direction. The current Sugiyama layered layout was
* designed for a single connected DAG; disconnected components were
* not in the original 11-circuit set.
V1 in1 0 AC=1
R1 in1 mid1 1k
C1 mid1 0 10n
R2 mid1 out1 1k
C2 out1 0 10n
* Independent stage (no shared nets except 0)
V2 in2 0 AC=1
R3 in2 out2 5k
C3 out2 0 1u
