* Binary-Weighted Capacitor DAC Array (PTM 28nm) [n2s test case 28]
*
* === n2s test-case metadata ===
* Source:       myadc repo, ptm28nm/dac/cap_dac_4p4.spice
*               (4+4 split capacitive DAC of an 8-bit SAR ADC)
* Circuit type: Passive binary-weighted capacitor DAC array
* Devices:      12 total -- 10 capacitor, 2 voltage source
* MOSFETs:      0
* Notes:        passive cap array; also exercises the .control-block skip
*               (its .control section contains destroy/let commands).
* Added:        2026-06-15  (real-world test-set enrichment)
* ===============================
* 4+4 Split-Capacitor DAC for 8-bit SAR ADC
* 28nm CMOS, VDD=0.9V, Vref=0.9V
*
* MSB array: 8Cu, 4Cu, 2Cu, Cu  (D7-D4)
* LSB array: 8Cu, 4Cu, 2Cu, Cu, Cu(dummy)  (D3-D0)
* Bridge cap: Cb = 16Cu/15 (ideal attenuation = 1/16)
*
* Unit capacitor Cu = 10fF
* Test: sweep all 256 codes, measure DAC output voltage

.param VDD=0.9
.param VREF=0.9
.param Cu=10f
.param Cb='16*Cu/15'

* Power supply & reference
VVDD vdd 0 DC VDD
VREF vref 0 DC VREF

***** Ideal switch model *****
.model sw_mod sw vt=0.45 vh=0.05 ron=10 roff=1T

*** MSB Array (D7-D4) ***
* Each cap bottom plate connects to VREF (bit=1) or GND (bit=0) via switches

* D7 (MSB) - 8Cu
S_D7H bm7 vref d7 0 sw_mod
S_D7L bm7 0   d7b 0 sw_mod
C_D7 top_msb bm7 '8*Cu'

* D6 - 4Cu
S_D6H bm6 vref d6 0 sw_mod
S_D6L bm6 0   d6b 0 sw_mod
C_D6 top_msb bm6 '4*Cu'

* D5 - 2Cu
S_D5H bm5 vref d5 0 sw_mod
S_D5L bm5 0   d5b 0 sw_mod
C_D5 top_msb bm5 '2*Cu'

* D4 - Cu
S_D4H bm4 vref d4 0 sw_mod
S_D4L bm4 0   d4b 0 sw_mod
C_D4 top_msb bm4 Cu

*** Bridge Capacitor ***
C_bridge top_msb top_lsb Cb

*** LSB Array (D3-D0) ***
* D3
S_D3H bl3 vref d3 0 sw_mod
S_D3L bl3 0   d3b 0 sw_mod
C_D3 top_lsb bl3 '8*Cu'

* D2
S_D2H bl2 vref d2 0 sw_mod
S_D2L bl2 0   d2b 0 sw_mod
C_D2 top_lsb bl2 '4*Cu'

* D1
S_D1H bl1 vref d1 0 sw_mod
S_D1L bl1 0   d1b 0 sw_mod
C_D1 top_lsb bl1 '2*Cu'

* D0 (LSB)
S_D0H bl0 vref d0 0 sw_mod
S_D0L bl0 0   d0b 0 sw_mod
C_D0 top_lsb bl0 Cu

* Dummy capacitor (always to GND)
C_DUMMY top_lsb 0 Cu

*** Control signals ***
* Use PWL to sweep all 256 codes
* Each code held for 10ns, total = 2560ns
* Bit voltages: 0.9V = logic 1, 0V = logic 0
* Complementary signals for switch control

.control
* Generate all 256 codes and measure output
destroy all

let num_codes = 256
let vout_array = vector(256)
let code = 0

* For each code, set up initial conditions and run operating point
foreach mycode 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 91 92 93 94 95 96 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 123 124 125 126 127 128 129 130 131 132 133 134 135 136 137 138 139 140 141 142 143 144 145 146 147 148 149 150 151 152 153 154 155 156 157 158 159 160 161 162 163 164 165 166 167 168 169 170 171 172 173 174 175 176 177 178 179 180 181 182 183 184 185 186 187 188 189 190 191 192 193 194 195 196 197 198 199 200 201 202 203 204 205 206 207 208 209 210 211 212 213 214 215 216 217 218 219 220 221 222 223 224 225 226 227 228 229 230 231 232 233 234 235 236 237 238 239 240 241 242 243 244 245 246 247 248 249 250 251 252 253 254 255
end

echo "Done"
.endc

.end
