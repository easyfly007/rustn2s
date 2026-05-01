* Deep Cascaded RC Signal Chain (10 stages)
* Stresses the Sugiyama placer with a long but trivially-linear DAG.
* Expected: a wide, shallow layout. Likely actual: same vertical-stack
* failure as 16_inverter_chain_5stage but on passive components, so
* the layout/routing quality is decoupled from the inverter-pair
* template.
V1 in 0 AC=1
R1  in   n1   1k
C1  n1   0    1n
R2  n1   n2   1k
C2  n2   0    1n
R3  n2   n3   1k
C3  n3   0    1n
R4  n3   n4   1k
C4  n4   0    1n
R5  n4   n5   1k
C5  n5   0    1n
R6  n5   n6   1k
C6  n6   0    1n
R7  n6   n7   1k
C7  n7   0    1n
R8  n7   n8   1k
C8  n8   0    1n
R9  n8   n9   1k
C9  n9   0    1n
R10 n9   out  1k
C10 out  0    1n
