* Folded Cascode Op-Amp (single-stage)
* Stresses the analyzer's pattern matcher: this is a textbook OTA
* topology that does NOT match the simple diff-pair / current-mirror
* / cascode template definitions because:
*   - The diff pair's drains feed into the FOLD nodes, which then
*     route through cascode pairs whose source is the fold node, not
*     drain-to-source matched to the diff-pair drain;
*   - Several PMOS cascodes share a common bias rather than being
*     diode-connected.
* What the analyzer recognizes here, vs. how the engineer would group
* it, is genuinely informative for audit item C3 ("topological
* assumptions in pattern recognition").
V1 vdd 0 1.8
V2 vssa 0 0
V3 vinp 0 AC=0.5
V4 vinm 0 AC=-0.5
* Tail current source
I1 vdd tail 100u
* Diff pair (PMOS input)
M1 fold_p vinp tail vdd pch W=20u L=0.5u
M2 fold_n vinm tail vdd pch W=20u L=0.5u
* Folding cascode (NMOS, sources at fold_p / fold_n)
M3 cas_p bn fold_p vssa nch W=10u L=0.5u
M4 cas_n bn fold_n vssa nch W=10u L=0.5u
* PMOS active load with cascode (top of the fold)
M5 cas_p bp top_p vdd pch W=20u L=0.5u
M6 cas_n bp top_n vdd pch W=20u L=0.5u
M7 top_p vbias vdd vdd pch W=10u L=0.5u
M8 top_n vbias vdd vdd pch W=10u L=0.5u
* NMOS current sink (bottom of the fold)
M9  fold_p bcas vsink vssa nch W=20u L=0.5u
M10 fold_n bcas vsink vssa nch W=20u L=0.5u
M11 vsink bn vssa vssa nch W=20u L=0.5u
* Bias references (kept as separate diode-connected legs)
M12 bn bn vssa vssa nch W=5u L=0.5u
M13 bp bp vdd vdd pch W=5u L=0.5u
* Output and load
C1 cas_n 0 1p
