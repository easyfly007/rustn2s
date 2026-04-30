# Netlist-to-Schematic 学习资料清单

这份清单收集与"从 SPICE / 网表自动生成可读原理图"这一问题相关的算法、论文、开源项目、教程和视频，按主题组织，便于系统性地学习并改进 `n2s` 工具。

每个条目都标注了它对应这个项目里的哪个模块/阶段，方便对照源码理解。

---

## 1. 问题概述

把网表（SPICE / Verilog netlist / EDIF）自动转换成人类可读的原理图（schematic）属于 **EDA（电子设计自动化）** 的子问题，工业界叫 **schematic generation / schematic recovery / netlist visualization**。它和 **graph drawing**（图绘制）、**VLSI placement & routing**（超大规模集成电路布局布线）共用很多算法。

核心子问题：

| 子问题 | 目的 | 对应到本项目模块 |
|---|---|---|
| 网表解析 | 从文本到结构化 device + net 列表 | `src/parser` |
| 模式识别 / 子电路识别 | 把 transistor 簇聚成功能块（diff pair, current mirror, …） | `src/analyzer` |
| 层次化布局 | 给器件分配 (x, y) 坐标，控制可读性 | `src/placer` |
| 布线 | 给每个 net 选 wire / label / power-symbol | `src/router` |
| 渲染 | SVG / KiCad / JSON 输出 | `src/export` |
| 质量评价 | 量化布局质量、驱动迭代优化 | `src/eval` |

---

## 2. 经典算法（必读 / 必学）

### 2.1 层次化图绘制（Sugiyama framework）

**这是 placer 的理论基础。** 把有向图分成多层、最小化交叉、再分配坐标，正是 `src/placer/mod.rs` 现在做的事。

- **K. Sugiyama, S. Tagawa, M. Toda, "Methods for Visual Understanding of Hierarchical System Structures", IEEE Trans. on Systems, Man, and Cybernetics, 1981.**
  四阶段框架的祖宗论文：cycle removal → layer assignment → crossing minimization → coordinate assignment。
- **Healy & Nikolov（编），《Handbook of Graph Drawing and Visualization》（CRC Press, 2013）**
  graph drawing 圣经；第 13 章 "Hierarchical Drawing Algorithms" 把 Sugiyama 框架讲得最清楚。免费 PDF 可在作者主页找到。
- **重子之 Coffman-Graham 层分配 / longest-path layering**：教材级介绍见 di Battista 等《Graph Drawing: Algorithms for the Visualization of Graphs》（Prentice Hall, 1999）。

**barycenter / median heuristic for crossing minimization**：
- M. J. Carpano, "Automatic display of hierarchized graphs for computer-aided decision analysis", IEEE Trans. SMC, 1980.
- P. Eades, N. C. Wormald, "Edge crossings in drawings of bipartite graphs", Algorithmica, 1994.

### 2.2 网格 / Manhattan 路由

**对应 `src/router` 以及 `docs/routing_improvement.md` 里规划的 A\* 改进。**

- **Lee algorithm**（C. Y. Lee, "An algorithm for path connections and its applications", IRE Trans. on Electronic Computers, 1961）— BFS 网格路由的最早形式。
- **A\* search**（Hart, Nilsson, Raphael, 1968）— 加启发式的 BFS，是 EDA / 游戏寻路的标配。
- **Hadlock's algorithm**、**Soukup's algorithm** — Lee 的剪枝优化。
- **Maze routing with rip-up and reroute**：S. Dutt, V. Shanmugavel, S. Trimberger, "Efficient incremental rerouting for fault reconfiguration in field programmable gate arrays", ICCAD 1999.
- **Channel routing**（`docs/routing_improvement.md` Phase C）：
  - A. Hashimoto, J. Stevens, "Wire Routing by Optimizing Channel Assignment within Large Apertures", DAC 1971.
  - T. Yoshimura, E. Kuh, "Efficient Algorithms for Channel Routing", IEEE Trans. on CAD, 1982 — 经典的 left-edge 算法。

### 2.3 力导向布局（force-directed）

适合追求对称性、紧凑性的小型电路；可以作为 Sugiyama 的对照思路或后处理。

- **T. Kamada, S. Kawai, "An algorithm for drawing general undirected graphs", Information Processing Letters, 1989.**
- **T. Fruchterman, E. Reingold, "Graph drawing by force-directed placement", Software: Practice and Experience, 1991.**
- **Stephen G. Kobourov, "Force-Directed Drawing Algorithms" (Handbook of Graph Drawing 第 12 章)**

### 2.4 模拟电路特有的模式识别

`src/analyzer` 现在的策略——优先识别 differential pair / current mirror / cascode / inverter——属于 **structural matching**。如果想做得更系统化：

- **R. A. Rutenbar, J. M. Cohn, "Layout tools for analog ICs and mixed-signal SoCs: a survey", DAC 2000** — 综述模拟电路自动布局领域。
- **F. Balasa, K. Lampaert, "Module placement for analog layout using the sequence-pair representation", DAC 1999.**
- **M. Meissner, L. Hedrich, "FEATS: Framework for Explorative Analog Topology Synthesis", DATE 2015** — 用图同构 + 子图匹配做模拟电路结构识别。
- **H. E. Graeb（编），《Analog Layout Synthesis: A Survey of Topological Approaches》（Springer 2011）**。

更工程的视角：**graph isomorphism / subgraph matching**：
- VF2 算法（Cordella et al., 2004）— `networkx` 里的 `subgraph_isomorphism` 用的就是它。
- nauty / Traces — McKay 的同构判定工具。

### 2.5 层次化（hierarchical）+ 子电路展开

- **J. Cohn 等，《Analog Device-Level Layout Automation》（Kluwer 1994）** — 经典模拟布局教材，里面有 hierarchical placement 的章节。

---

## 3. 开源软件 / 可读源码

按相关度从高到低：

### 3.1 高度相关：直接做 netlist → schematic

- **MySchematic (C++)** — 本项目的 C++ 前身。`docs/architecture.md` 里有 Rust 模块到 C++ 模块的对照表，是"参考实现"。
- **schemdraw**（Python，作者 Cole Williams）— <https://schemdraw.readthedocs.io/>。Pythonic API 画原理图，符号库丰富，没有 placer/router 但符号绘制思路可参考。
- **SKiDL**（Python）— <https://github.com/devbisme/skidl>。代码即网表，能导出 KiCad；它的 `Network` / `Bus` 抽象值得借鉴。
- **Lcapy**（Python）— <https://github.com/mph-/lcapy>。把 SPICE 风格网表渲染成 LaTeX/PNG，靠 `circuitikz`；其 schematic placement 用了 graph layout 思路。
- **netlistsvg**（JavaScript / Node）— <https://github.com/nturley/netlistsvg>。把 Yosys 输出的 JSON netlist 渲染成 SVG；用 ELK（Eclipse Layout Kernel）做布局，是看 ELK 在数字逻辑上效果的最好入口。

### 3.2 EDA 主流工具（重点看其 placement / routing 模块）

- **KiCad** — <https://gitlab.com/kicad/code/kicad>。`pcbnew/router` 用的是 PNS（push-and-shove）路由器，看它怎么处理 obstacle avoidance。`eeschema` 是原理图编辑器，看 KiCad 怎么序列化 `.kicad_sch`（本项目的 export 已经在用）。
- **OpenROAD** — <https://github.com/The-OpenROAD-Project/OpenROAD>。开源数字 RTL-to-GDS 流程。`OpenROAD/src/grt` (global router)、`OpenROAD/src/dpl` (detailed placer) 是研究实现的好材料。
- **Magic VLSI** — <http://opencircuitdesign.com/magic/>。老牌版图工具，源码里有 channel routing 实现。
- **Qucs / Qucs-S** — <https://github.com/Qucs/qucs>、<https://github.com/ra3xdh/qucs_s>。模拟电路仿真器，自带原理图编辑；其 schematic file format 和导出代码可以学。

### 3.3 图布局库（直接用或抄它的算法）

- **Graphviz** — <https://gitlab.com/graphviz/graphviz>。`dot` 工具用的是 **Gansner-North-Vo (GNV)** 算法，本质是 Sugiyama 框架的工业级实现。论文：E. Gansner, E. Koutsofios, S. North, K.-P. Vo, "A Technique for Drawing Directed Graphs", IEEE TSE, 1993。**强烈推荐精读这篇 22 页的论文，它就是把 Sugiyama 完整工程化的范本。**
- **ELK (Eclipse Layout Kernel)** — <https://github.com/eclipse/elk>。Java 实现的布局引擎，netlistsvg 用的就是它；自带 layered（Sugiyama）、force、stress、tree 等多种算法。
- **OGDF (Open Graph Drawing Framework)** — <https://github.com/ogdf/ogdf>。C++ 算法库，论文里见到的算法基本都有实现。
- **dagre** — <https://github.com/dagrejs/dagre>。JS 版 Sugiyama，源码短，适合快速学习每一步在做什么。

### 3.4 Rust 生态（与本项目直接对接）

- **petgraph** — <https://github.com/petgraph/petgraph>。Rust 图库，有基本的 DAG / 拓扑排序，但没有完整的 layered layout。
- **layout** — <https://github.com/nadavrot/layout>（Nadav Rotem）。Rust 写的 layered graph drawing，输出 SVG；体量小，源码可读，思路接近本项目的 placer。
- **lyon** — <https://github.com/nical/lyon>。2D path tessellation，如果以后想自己做 SVG 渲染优化可以参考。
- **yosys**（虽然是 C++）和 **prjunnamed**（Rust）的 netlist IR 是看"如何把网表内部表示得可被多种后端消费"的好例子。

### 3.5 SPICE 解析

- **ngspice** — <https://ngspice.sourceforge.io/>。SPICE 仿真器，源码里有完整的 netlist parser，包括所有边角语法。
- **PySpice** — <https://github.com/FabriceSalvaire/PySpice>。Python 包装，里面也有自己的 netlist 类。

---

## 4. 教程 / 课程

### 4.1 课件 / 在线讲义

- **CMU 18-447 / Berkeley EE C247B / Stanford EE371**：找 "VLSI CAD" / "Physical Design" 关键字。Andrew Kahng（UCSD）和 Igor Markov（Michigan / Google）的物理设计课件公开度最高。
  - **Andrew Kahng, Jens Lienig, Igor Markov, Jin Hu, 《VLSI Physical Design: From Graph Partitioning to Timing Closure》（Springer 2011）** — 这本书是物理设计领域的标准教材，第 4 章讲 placement，第 5 章讲 routing。**最推荐先看这本。**
- **Sherwani, 《Algorithms for VLSI Physical Design Automation》（Kluwer 1999）** — 老一些但讲算法很细。
- **Naveed Sherwani / Andrew Kahng 等，"VLSI Physical Design Automation: Theory and Practice"** — 经典综述。

### 4.2 Coursera / edX / 学校公开课

- **Coursera "VLSI CAD Part I: Logic"** & **Part II: Layout**（Rob A. Rutenbar，UIUC）— 免费旁听。Part II 有专门讲 placement 和 routing 的视频。
- **NPTEL "VLSI Physical Design"**（IIT 印度）— YouTube 上有完整视频。
- **MIT OCW 6.111 / 6.374** — 数字 VLSI；不直接是布局算法但能帮你理解上下文。

### 4.3 博客 / 文章

- **Will Crichton 的图布局系列**（[willcrichton.net](https://willcrichton.net)）。
- **The Graphviz tutorial** — <https://graphviz.org/documentation/>。
- **OpenROAD 文档站**：<https://openroad.readthedocs.io>。

---

## 5. YouTube / 视频

- **NPTEL — "VLSI Physical Design"** — 完整课程，搜频道 `nptelhrd` 加关键字。
- **CMU "Computer Architecture" / "VLSI CAD"** — Onur Mutlu / Larry Pileggi 的讲座。
- **"Graph Drawing"** 关键字下的会议（GD'XX）演讲在 YouTube 上有很多，特别是 Stephen Kobourov、Michael Kaufmann 的报告。
- **"Eclipse Layout Kernel" 演讲** — 搜 "ELK Eclipse Layout" 找到 EclipseCon 上 Reinhard von Hanxleden 团队的讲解。
- **Yosys / OpenROAD workshop 录像** — Github action 视频里有，能看到工业流程怎么跑。
- **"KiCad Push and Shove Router"** — Tomasz Wlostowski 在 FOSDEM 上做过演讲。

---

## 6. 模拟 IC 布局专题（更深入做 analog 时再啃）

- **Helmut E. Graeb（编），《Analog Layout Synthesis: A Survey of Topological Approaches》(Springer 2011)**。
- **Lampaert, Gielen, Sansen, 《Analog Layout Generation for Performance and Manufacturability》（Kluwer 1999）**。
- **DAC / ICCAD / DATE / ASP-DAC** 这四大会议每届都有 analog layout 的 session。Google Scholar 搜 "analog placement" / "analog routing" 排序 by date。

---

## 7. 推荐学习路径

按从基础到深入排：

1. **Gansner-North-Vo 1993 paper（Graphviz dot）** — 一周把 Sugiyama 框架的工程实现搞透。
2. **Kahng/Lienig 教材第 4-5 章** — 把 placement / routing 的标准词汇和算法过一遍。
3. **dagre 源码 + ELK 文档** — 看现代实现怎么做。
4. **netlistsvg + schemdraw 源码** — 看具体到 netlist → 原理图渲染时这些算法长什么样。
5. **OpenROAD `dpl` / `grt` 源码** — 看工业级 placer / router。
6. **GD（Graph Drawing）会议每年精选 2-3 篇 paper** — 跟踪前沿。
7. **专攻方向再选 analog layout 论文集** 或 **A\* / channel routing 论文**。

---

## 8. 与本项目的对应关系（速查）

| 想改进 | 看哪些资料 |
|---|---|
| `placer` 的 layer assignment 更聪明 | Gansner-North-Vo paper, Kahng 教材 §4 |
| `placer` 减少 crossing | Eades-Wormald paper, dagre 源码 |
| `placer` 试 force-directed | Fruchterman-Reingold paper, Kobourov chapter |
| `router` A\* 实现（`docs/routing_improvement.md` Phase B） | Hart-Nilsson-Raphael 1968, Red Blob Games 的 A\* 教程 |
| `router` channel routing（Phase C） | Yoshimura-Kuh 1982, Magic VLSI 源码 |
| `analyzer` 模式识别更通用 | VF2 (Cordella 2004), Meissner-Hedrich FEATS, networkx subgraph 文档 |
| `eval` 评分更接近人眼 | GD 会议里 "aesthetic criteria" 的论文 |
| `export` 渲染优化 | 看 Graphviz 和 KiCad 的 SVG/sch 写出器 |

---

## 9. 不在这份清单里、但你可能会查到的资源

- 数字 RTL synthesis（Yosys、Synopsys DC）— 离本项目较远，先跳过。
- PCB autorouting（FreeRouting 等）— 解决的是 PCB 走线，不是原理图布局。
- Hardware description language 教程（Verilog/VHDL）— 上游问题，不是本项目要解的。

---

如果学习中发现某个资料特别有用、或想加新条目，直接编辑这份文档；如果某条变 dead link，删掉别留。
