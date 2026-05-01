* Star Fanout
* One driver feeds eight resistive loads on a single net, exercising
* the MST router's behavior on high-fanout nets. The 11-circuit set
* had at most 4 pins per net.
V1 in 0 AC=1
* Single driver resistor
R0 in star 100
* Eight identical loads off the star node
R1 star 0 1k
R2 star 0 1k
R3 star 0 1k
R4 star 0 1k
R5 star 0 1k
R6 star 0 1k
R7 star 0 1k
R8 star 0 1k
