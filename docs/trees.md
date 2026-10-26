Tree allometry for a simulation: verified equations, empirical ranges, and limits
Notation: (D) = DBH (usually measured at 1.3 m; specify whether cm or m), (H) = total height, (h) = height above ground, (CR) = crown radius, (CW) = crown width, and (HCB) = height to crown base. Units matter: empirical coefficients are only meaningful with the units and population used to fit them.

The central modeling caution is that these are different relationships: pipe-model theory links foliage to conductive sapwood; height–DBH and crown–DBH are empirical allometries; taper describes stem diameter along height; biomass equations are fitted to harvested trees. They are not interchangeable universal laws.

1. Pipe model theory and da Vinci’s rule
Pipe model theory (PMT)
The original paper is Shinozaki, K., Yoda, K., Hozumi, K. & Kira, T. (1964), “A quantitative analysis of plant form—The pipe model theory. I. Basic analyses,” Japanese Journal of Ecology 14:97–105. Its DOI is 10.18960/seitai.14.3_97. I did not retrieve the original paper itself; the following formulation is quoted and discussed in the accessible review by Lehnebach et al. (2018), Annals of Botany 121:773–795, DOI 10.1093/aob/mcx194.

The review quotes the theory’s core idea: “a unit amount of leaves is provided with a pipe whose thickness or cross-sectional area is constant.” The pipe is conceived to run from those leaves down through branches and stem. In a useful model shorthand:

[ A_{\rm sapwood}(z) \propto F_{\rm above}(z) ]

where (F_{\rm above}(z)) is the foliage supported above the relevant section in the tree’s branching structure. This is not a general rule that total trunk cross-sectional area is proportional to total foliage. The review cautions that the hydraulic interpretation concerns the conductive portion of the stem, and that the PMT’s properties are not valid as universal rules; sapwood–foliage relationships vary with environment and ontogeny.

Da Vinci’s area-preserving rule
Minamino & Tateno (2014), “Tree Branching: Leonardo da Vinci’s Rule versus Biomechanical Models,” PLOS ONE 9(4):e93535, DOI 10.1371/journal.pone.0093535, define the rule as the sum of branch cross-sectional areas above a branching point equaling the cross-sectional area immediately below it:

[ A_{\rm parent}=\sum_{j=1}^{n}A_{{\rm daughter},j} \quad\Longleftrightarrow\quad r_{\rm parent}^{,2}=\sum_{j=1}^{n}r_{{\rm daughter},j}^{,2} ]

for circular cross-sections. It is areas, not diameters, that add. The study measured Fagus crenata and Abies homolepis: departures from the rule increased in model calculations as lateral-branch weights or branching angles increased. The authors report that the departure was small for the natural branching patterns they measured, but do not establish the rule as an exact invariant.

2. WBE / metabolic scaling and the height–diameter prediction
What the quarter-power results do—and do not—say
West, Brown & Enquist (1997), “A general model for the origin of allometric scaling laws in biology,” Science 276(5309):122–126, DOI 10.1126/science.276.5309.122, is the foundational WBE model. Its headline scaling is metabolic rate (B\propto M^{3/4}). WBE’s 1999 plant-vascular paper is West, Brown & Enquist, “A general model for the structure and allometry of plant vascular systems,” Nature 400:664–667, DOI 10.1038/23251. A later accessible plant-allometry paper explicitly identifies these 1997/1999 WBE models and discusses their assumptions and predictions: Price et al. (2022), Plant Physiology 190:1854–1865, DOI 10.1093/plphys/kiac358.

For trees, it is important not to equate the 3/4 metabolic exponent with a 3/4 height–DBH exponent. The tree-height prediction discussed in the allometry literature is:

[ H\propto D^{2/3} ]

Hulshof, Swenson & Weiser (2015) describe the (2/3) prediction as arising from elastic similarity / metabolic-scaling arguments. Price et al. give the associated elastic-similarity maximum-height relation (l_{\max}\propto r^{2/3}), under assumed constant material properties. This is a theoretical prediction, not a universal fitted forest equation.

A separate 1999 paper is directly useful for plant production: Enquist, West, Charnov & Brown, “Allometric scaling of production and life-history variation in vascular plants,” Nature 401:907–911 (1999). Its accessible institutional record states (dM/dt\sim M^{3/4}) and relative growth rate ((1/M)(dM/dt)\sim M^{-1/4}): University of New Mexico record.

Empirical height–DBH slopes are not the theoretical value
Hulshof, C.M., Swenson, N.G. & Weiser, M.D. (2015), “Tree height–diameter allometry across the United States,” Ecology and Evolution 5(6):1193–1204, DOI 10.1002/ece3.1328, analyzed 293 species. In the power-law fits, the reported height-on-diameter slopes were 0.53 for angiosperms and 0.60 for gymnosperms, versus the (2/3) prediction. The authors found the Gompertz function fit best overall, so those power-law slopes should be treated as summaries over the data range, not fixed species constants.

3. Published height, crown and crown-ratio relationships
UK broadleaf species: equations and downloadable fitted parameters
Evans, M.R. et al. (2015), “Allometry and growth of eight tree taxa in United Kingdom woodlands,” Scientific Data 2:150006, DOI 10.1038/sdata.2015.6, reports UK equations for eight taxa, including European beech (Fagus sylvatica), pedunculate oak (Quercus robur), ash, sycamore, field maple, birch, hazel and hawthorn.

For adult trees, the reported forms are:

[ H=1.35+(\mathrm{maxH}-1.35)(1-e^{-bD}) ]

[ CR=aD^b ]

[ CH=aH^b ]

Here (CH) is crown depth/length (top of tree to crown base), so a corresponding crown ratio is (CH/H). Given the reported crown-depth power function, the implied ratio is (aH^{b-1}). This last expression is a direct algebraic consequence of their model, not a separately fitted equation. The height equation has an asymptote at maxH; the paper distinguishes adult trees ((DBH>10) cm) from saplings.

The paper’s parameter file is Data record 7. Dryad confirms that it contains the species-specific SORTIE parameters needed to instantiate the equations: Dryad dataset, DOI 10.5061/dryad.2c1s7. I could read the equation forms and verify that the fitted parameter file exists, but could not retrieve the parameter values from the file, so I do not quote species-specific coefficients.

Temperate broadleaf and conifer crown relationships
For Norway spruce (Picea abies) and European beech, Sharma, R.P., Vacek, Z. & Vacek, S. (2017), “Modelling tree crown-to-bole diameter ratio for Norway spruce and European beech,” Silva Fennica 51(5), article 1740, DOI 10.14214/sf.1740, define:

[ CDBDR=\frac{\text{maximum crown width}}{DBH}. ]

The study reports conditional (R^2\approx0.73) for spruce and (0.78) for beech for its spatially explicit mixed-effects models. It found the ratio depended on more than DBH: significant predictors included DBH, height to crown base, dominant height, competition, and species proportion. That is a warning against using one fixed crown-width/DBH ratio in a stand simulation. The accessible page identifies a model-parameter table, but I could not retrieve its numeric coefficients or species-specific predicted ratio values.

For a DBH-only radius power law, (CR=aD^b), the ratio is not constant:

[ \frac{CR}{D}=aD^{b-1},\qquad \frac{CW}{D}=2aD^{b-1}\quad\text{if }CW=2CR. ]

Thus, a constant crown-radius/DBH ratio implicitly assumes (b=1).

A further UK allometry source is Peper, McPherson & Mori (2001), “Equations for predicting diameter, height, crown width, and leaf area of San Joaquin Valley street trees,” Journal of Arboriculture 27:306–317. The US Forest Service record confirms the study measured 12 species and reports adjusted (R^2>0.70) for height, crown diameter and leaf area against DBH, but the readable record does not expose the coefficient table: USFS record.

A reported height/DBH ratio—not a universal allometry
Chakraborty, Saha & Reif (2016), “Biomass equations for European beech growing on dry sites,” iForest 9:751–757, DOI 10.3832/ifor1881-009, report that their taller dry-site beech sample had mean DBH 8.7 cm, mean height 9 m, and a reported mean height/DBH ratio of 112. This is the authors’ reported ratio statistic for their sample—not a universal species constant or necessarily the ratio of the two means.

4. Biomass equations with actual coefficients
For European beech on dry sites, the same Chakraborty et al. study harvested 66 trees in its DBH category (DBH range 2–18 cm). Its fitted dry above-ground biomass equation is:

[ AGB_{\rm dry}(\mathrm{kg})=0.201,DBH^{2.173} ]

with DBH in cm; the reported adjusted (R^2) is 0.94. Their component equations are:

[ \begin{aligned} \text{stem with bark, dry (kg)} &=0.100,DBH^{2.343}\ \text{branches plus foliage, dry (kg)} &=0.123,DBH^{1.776}. \end{aligned} ]

They also report a two-predictor log-scale model:

[ \mathrm{Log}(AGB)=1.711,\mathrm{Log}(DBH) +0.680,\mathrm{Log}(H)-0.918. ]

The article prints Log, not ln, in this equation. These are site- and size-range-specific dry-site beech estimates, not general coefficients for all beech stands.

He et al. (2018), “Allometric biomass equations for 12 tree species in coniferous and broadleaved mixed forests, Northeastern China,” PLOS ONE 13:e0186226, DOI 10.1371/journal.pone.0186226, is another temperate multi-species source; it fits species-specific DBH biomass power functions (Y=aD^b). I could verify the study design and equation form, but not extract its coefficient tables here.

5. Trunk taper and form factor
Wilms et al. (2024), “Applying taper function models for black locust plantations in Greek post-mining areas,” Scientific Reports 14:13557, DOI 10.1038/s41598-024-63048-1, describe taper as a profile varying from base to tip and state that a stem is often approximated in sections: lower neiloid, middle paraboloid, upper cone. Their study compared 19 fixed-effect functions on 30 destructively sampled black locust trees (DBH 3–22 cm); the Kozak (2004) equation performed best for their local dataset. This supports using calibrated, species- and site-appropriate taper, rather than assuming one profile is exact for every tree.

For a compact simulation, the following is a proposed simple parameterization, not a fitted universal law:

[ d(h)=D\left(\frac{1-h/H}{1-1.3/H}\right)^p, \qquad 1.3\le h\le H. ]

It returns (d(1.3)=D) and tapers to zero at the tip. (p) is a shape parameter to fit from measured upper-stem diameters; (p=1) gives a conical profile, while (p=1/2) gives a paraboloid-like profile because cross-sectional area then declines linearly with height in this idealized form. Do not treat either (p) value as a measured “typical tree” coefficient. The accessible source supports the segmented shapes, not a single exponent for real temperate trees.

A common volume summary is:

[ V=f\left(\frac{\pi D^2}{4}\right)H, ]

where (f) is the form factor relative to the breast-height basal area. Real-tree (f) depends on species, size and stand conditions; I did not verify a broadly applicable numeric value. The 2024 taper paper notes that a form-factor method can estimate total stem volume but is less useful for log or stem-section volumes.

I could not verify a specific Metz/Metzger taper equation from a readable fetched source, so I have not supplied one. Likewise, the numeric crown-model coefficients for the UK taxa and the Czech spruce/beech crown-ratio models were not available in the extracted pages; their linked records/articles are the appropriate places to obtain those fitted values.

Research brief: incremental tree growth and structural models
1. Tree height/diameter growth through time
Chapman–Richards: equation and peak-growth age
A convenient Chapman–Richards form for either height or diameter as a function of age is

[ Y(a)=A\left(1-e^{-ka}\right)^p, ]

where (Y) is height (H) or diameter (D), (A) is the asymptotic size, (k) is a rate parameter, and (p) controls curve shape. This zero-at-age-zero form is widely used; another common form is

[ Y(a)=A\left(1-b e^{-ka}\right)^p, ]

which allows a nonzero initial size when (b\ne1). Notation varies between forestry papers, so fitted parameters must be interpreted using the equation specified by each paper.

For (p>1), the age at maximum annual increment (maximum (dY/da), the inflection point of the size curve) is

[ a_{\mathrm{peak}}=\frac{\ln p}{k} ]

for the first form, or (\ln(bp)/k) for the second. If (p\leq1), the first form has no positive-age interior maximum in its derivative. These peak-age expressions are derived by differentiating the equations; there is no single universal peak age. It depends on the fitted rate and shape parameters.

A useful forestry reference is Desta Fekedulegn, Mairitin P. Mac Siurtain, and Jim J. Colbert, “Parameter estimation of nonlinear growth models in forestry,” Silva Fennica 33(4), article 653 (1999), pp. 327–336, DOI 10.14214/sf.653. Its abstract says that Chapman–Richards and von Bertalanffy model equations and derivatives are presented, and that parameters were fitted to Norway spruce top height versus age. I could read the journal page and abstract, but the full-text PDF is not available through the page extraction, so I could not check its equation tables or parameter estimates. Journal page

The foundational forestry reference is L. V. Pienaar and K. J. Turnbull, “The Chapman-Richards Generalization of Von Bertalanffy’s Growth Model for Basal Area Growth and Yield in Even-Aged Stands,” Forest Science 19(1), 2–22 (1973), DOI 10.1093/forestscience/19.1.2. The bibliographic record is available, but the publisher page returned 403, so I did not verify its full text or page-specific equations.

von Bertalanffy and von Bertalanffy–Richards
The classical fixed-shape von Bertalanffy size-age form can be written

[ Y(a)=A\left[1-e^{-k(a-a_0)}\right]^3. ]

For that form, maximum increment occurs at

[ a_{\mathrm{peak}}=a_0+\frac{\ln 3}{k}. ]

The von Bertalanffy–Richards form generalizes the shape exponent; with (p) replacing 3, the peak is (a_0+\ln(p)/k). These formulas describe the increment peak, not the age at which the tree reaches its final height or diameter.

A useful empirical example is Elvis Felipe Elli et al., “Climatic factors defining the height growth curve of forest species,” iForest – Biogeosciences and Forestry 10(3), 547–553 (2017), DOI 10.3832/ifor2189-010. They measured five species, selecting 18 trees per species, with repeated height measurements from 2008 to 2014. Their Table 1 reports the following raw coefficients for the von Bertalanffy–Richards fits:

Species	(\beta_0)	(\beta_1)	(\beta_2)	Fit CV
Parapiptadenia rigida	43.7913	0.000053	0.8691	34.42%
Peltophorum dubium	42.0389	0.000055	0.8688	23.97%
Eucalyptus urophylla × E. grandis	26.1731	0.00078	1.3692	16.23%
The model was selected as the best fit for these three species in that study. These are not portable “typical” Chapman–Richards parameters: the main article points to supplementary Table S1 for model equations, and I could not verify the precise coefficient-to-equation mapping or time scaling from the extracted page. Accordingly, I cannot reliably calculate peak ages from these coefficients. The values above are copied from Table 1, not reinterpreted as asymptotes or rate constants. The article page spans pp. 547–553; the data, model and fit information are in its methods/results and Table 1. Full article

Simulation takeaway: use a height-age or diameter-age equation calibrated for the species, site, and time units in question. Treat asymptote/rate/shape values as fitted quantities, and compute the increment peak from that exact equation—not from the model name alone.

2. Biomass allocation, sink strength, and structural models
GreenLab
GreenLab combines organ development with plant-level biomass production. In the review’s notation, plant production is (Q(t)), plant demand is (D(t)), and organs have sink-strength functions and organ cohorts. The review describes biomass from a common pool as distributed in proportion to sink strength. A transparent algebraic rendering of that rule is

[ D_o(t)=\sum_{c\in o}P_o f_o(j_c),\qquad D(t)=\sum_oD_o(t),\qquad \Delta B_o(t)=Q(t)\frac{D_o(t)}{D(t)} , ]

where (P_o) is the sink-strength value for organ type (o), (f_o(j)) is its expansion-related sink variation at organ age (j), and the sum is over active organs/cohorts. This is a mathematical restatement of the described proportional-allocation rule, not a verbatim transcription of a rendered equation.

Philippe de Reffye, Baogang Hu, Mengzhen Kang, Véronique Letort, and Marc Jaeger, “Two decades of research with the GreenLab model in agronomy,” Annals of Botany 127(3), 281–295 (2020), DOI 10.1093/aob/mcaa172, describes GreenLab as a discrete dynamic FSPM and discusses sink organs, source organs, organ cohorts, and plant demand. PMC article

A second GreenLab paper makes the allocation principle explicit: it says GreenLab “does not allocate biomass to organs or organ groups according to fixed partitioning tables, but according to the current, relative sink strength of the organs that are expanding.” Yuntao Ma et al., “Parameter Stability of the Functional–Structural Plant Model GREENLAB as Affected by Variation within Populations, among Seasons and among Growth Stages,” Annals of Botany 99(1), 61–73 (2007; published online 2006), DOI 10.1093/aob/mcl245. In its maize implementation, leaf-blade sink strength is normalized to 1, and the model includes sinks for blade, sheath, internode, pith, cob, and tassel. Those are maize organ classes, not a universal allocation recipe for a woody tree. PMC article

For a tree simulation, the modeler would need to define which organs represent foliage, trunk wood, branch wood, roots, and reproductive organs, and give them suitable sink strengths and expansion histories. GreenLab does not prescribe universal fixed percentages for trunk versus branches versus foliage.

L-systems, L-Py, and L-PEACH
An L-system provides a way to generate plant architecture step by step through rewriting rules. In L-Py, plant components are modules in a bracketed string; rules transform the current string into the next one, and geometric interpretation produces the 3-D structure. L-Py itself is a modeling framework, not a particular carbon-allocation rule: physiological functions and allocation equations must be supplied by the model.

Frédéric Boudon, Christophe Pradal, Thomas Cokelaer, Przemyslaw Prusinkiewicz, and Christophe Godin, “L-Py: An L-System Simulation Framework for Modeling Plant Architecture Development Based on a Dynamic Language,” Frontiers in Plant Science 3, article 76 (2012), DOI 10.3389/fpls.2012.00076. The paper’s L-Py code example uses 137.5° and 40° as rotation/insertion values. They are illustrative code parameters, not estimates of a general biological branch-angle distribution. PMC article

For a source–sink tree example, L-PEACH combines L-system architecture with carbon allocation and carbohydrate flow among individual organs. The primary reference is Mitch T. Allen, Przemyslaw Prusinkiewicz, and Theodore M. DeJong, “Using L-systems for modeling source-sink interactions, architecture and physiology of growing trees: the L-PEACH model,” New Phytologist 166, 869–880 (2005), DOI 10.1111/j.1469-8137.2005.01348.x. The primary PDF was not extractable in this research run; the citation and model description were identified from bibliographic results, while the GreenLab review also lists L-PEACH among organ-level source–sink FSPMs.

Model-design implication: use L-systems for the incremental topology/geometry if that suits your implementation; add an explicit source–sink model if biomass should determine organ growth. Do not assume an L-system grammar, by itself, determines what proportion of new biomass goes into trunk, branch, or leaf.

3. Self-thinning and stand density
Yoda’s (-3/2) rule
The conventional self-thinning relation is

[ \bar w=C\rho^{-3/2}, \qquad \log\bar w=\log C-\frac32\log\rho, ]

where (\bar w) is mean individual plant mass and (\rho) is plant density in a crowded, even-aged stand. The (-3/2) is the slope of mean individual mass against density in log–log space.

The original reference is K. Yoda, T. Kira, H. Ogawa, and K. Hozumi, “Self-thinning in overcrowded pure stands under cultivated and natural conditions (Intraspecific competition among higher plants XI),” Journal of Biology, Osaka City University 14, 107–129 (1963). I did not retrieve the original article text, and it has no DOI identified in the results. The equation above is the conventional expression of the rule; I could not check its original notation or page location against Yoda et al.’s paper.

Reineke stand density index
Reineke’s size–density relation is conventionally represented as

[ \log N=\alpha-1.605\log D_q, ]

where (N) is stems per unit area and (D_q) is quadratic mean diameter. The corresponding normalized stand density index (SDI) is

[ \mathrm{SDI}=N\left(\frac{D_q}{D_{\mathrm{ref}}}\right)^{1.605}. ]

Use consistent units: a common reference is (D_{\mathrm{ref}}=25) cm (approximately 10 inches). Thus the size–density line has slope (-1.605); the SDI normalization uses the positive exponent (+1.605).

The original reference is L. H. Reineke, “Perfecting a stand-density index for even-aged forest,” Journal of Agricultural Research 46, 627–638 (1933). The US Forest Service record confirms the article’s citation and explains that the index is based on number of trees and average diameter, but its extracted page does not show the equation. [US Forest

Research synthesis: trees in 8 m voxels with subvoxel geometry
Bottom line
An 8 m voxel is a useful container for tree data, but not a credible tree-shaped primitive. Use the coarse voxel to store semantics and aggregate properties; use subvoxels or a separate crown/skeleton representation to retain shape.

The sources support a qualified answer on detail:

Keep the crown’s outer form and major branching structure. A large perceptual study found branch angles, branch lengths and branch widths to be important to perceived tree realism.
Simplify small branches and individual foliage. Published LOD work progressively replaces branch meshes with translucent lines and ultimately a few lines. A separate tree representation abstracts foliage into repeatable “lobe-textures.”
But there is no universal measured minimum number of branches, crown lobes, or subvoxels at which a tree “reads as a tree.” The located studies do not establish a perception threshold for an 8 m grid. The subvoxel choices below are therefore engineering starting points, not experimentally validated perceptual cutoffs.
For physical contents, one detailed crown-volume study estimated that 95.3–98.5% of sampled crown volume was void space under its explicit definition of leaf and branch volume. That is a useful benchmark for an honest volume-occupancy model—but it is not the same thing as a photograph’s visual canopy cover, optical gap fraction, or the fraction of coarse grid cells touched by LiDAR returns.

1. How much detail should survive at low resolution?
Evidence from perception and LOD work
ICTree: Automatic Perceptual Metrics for Tree Models — Tomas Polášek, David Hrůša, Bedřich Beneš and Martin Čadík, 2021, ACM Transactions on Graphics 40(6), 1–15.
DOI: 10.1145/3478513.3480519 · Project page · University publication record

The authors report a user study in which 4,000 participants compared over one million pairs of images. The resulting predictors use tree geometry or images. Their finding about shape features is direct:

“branching angles, length of branches, and widths are critical for perceived realism.”

This is strong support for retaining the structure and proportions of the main branches—not necessarily every small branch. It does not directly test “silhouette versus branch detail,” nor does it state a universal minimum branch count or voxel size.

Tree Branch Level of Detail Models for Forest Navigation — Xiaopeng Zhang, Guanbo Bao, Weiliang Meng, Marc Jaeger, Hongjun Li, Oliver Deussen and Baoquan Chen, 2017, Computer Graphics Forum 36(8), 402–417.
DOI: 10.1111/cgf.13088 · KOPS publication page

This is a practical, branch-focused LOD method. It starts from a tree skeleton and fits branch meshes of different densities. The description says:

“For distant models, these branch meshes are gradually replaced with semi-transparent lines until the tree recedes to a few lines.”

This supports progressive simplification, but it is a rendering-performance method rather than a user study defining a universal perception threshold.

Texture-lobes for Tree Modelling — Yotam Livny, Soeren Pirk, Zhanglin Cheng, Feilong Yan, Oliver Deussen, Daniel Cohen-Or and Baoquan Chen, 2011, ACM SIGGRAPH 2011 Papers.
DOI: 10.1145/1964921.1964948 · Stanford Graphics Laboratory page

This method encodes foliage as canonical structures called “lobe-textures,” then synthesizes detailed trees from that lighter representation. The paper’s useful design implication is that foliage can be represented by crown clumps/lobes rather than leaf-by-leaf geometry while still aiming to visually resemble the input.

Takeaway for a coarse grid
The evidence supports a hierarchy like this:

Preserve the crown envelope and crown profile: broad, narrow, rounded, conical, irregular, etc.
Preserve trunk placement and primary branch direction, length and relative width where the view makes them visible.
Represent fine foliage with clumps/lobes, texture, or density, rather than individual leaves.
Fade secondary branches and fine foliage detail with distance.
The claim that “silhouette always matters more than branches” is plausible as a design rule, but I did not find a measured universal ranking or threshold that proves it. ICTree identifies branching angles, lengths and widths as critical; Zhang et al. demonstrate a branch-LOD scheme but do not report a minimum detail threshold.

2. Published tree-generation, reconstruction, voxel and mesh methods
Procedural growth and tree form
Modeling Trees with a Space Colonization Algorithm — Adam Runions, Brendan Lane and Przemyslaw Prusinkiewicz, 2007, Eurographics Workshop on Natural Phenomena, pp. 63–70.
DOI: 10.2312/NPH/NPH07/063-070 · Algorithmic Botany publication page

The paper extends the space-colonization idea to 3D tree structures. Its abstract says the method produces “surprisingly realistic tree structures” and that parameters correspond to “visually relevant tree characteristics” identified in landscaping. It is a reasonable way to generate a plausible skeleton and crown architecture before turning it into branch tubes, meshes, or subvoxel occupancy.

Reconstruction from point clouds and skeletons
Automatic Reconstruction of Tree Skeletal Structures from Point Clouds — Yotam Livny, Feilong Yan, Matt Olson, Baoquan Chen, Hao Zhang and Jihad El-Sana, 2010, Proceedings of ACM SIGGRAPH Asia 2010.
DOI: 10.1145/1866158.1866177 · Ben-Gurion University publication record

This method uses active laser scanning and global optimizations to reconstruct tree skeletons from sparse, incomplete and noisy point data; the authors note that it can reconstruct multiple overlapping trees without segmentation. It is a useful reference if subvoxel branch geometry is to be reconstructed rather than procedurally generated.

Voxelization for canopy structure and tree-space analysis
Exploring trees in three dimensions: VoxR, a novel voxel-based R package dedicated to analysing the complex arrangement of tree crowns — Bastien Lecigne, Sylvain Delagrange and Christian Messier, 2018, Annals of Botany 121(4), 589–601.
DOI: 10.1093/aob/mcx095 · Full text at PMC

VoxR treats a voxel as a portion of space explored by a tree and uses occupied voxels to describe spatial architecture. The paper shows voxelized point clouds at 0.1 m, 0.5 m and 1 m resolutions. Its methods also test voxel sizes of 0.01–0.15 m for filled cylinders and 0.05–0.2 m for empty cylinders when studying occlusion compensation. The authors’ conclusion is that larger voxels can help compensate for occlusion, “at the expense of metrics precision in some cases.”

Those resolutions are examples from tree-scanning and architecture analysis—not validated settings for visualizing an 8 m city grid. VoxR’s central idea is nevertheless useful: store which subvolumes the crown explores, and optionally use point counts or another density measure within them.

Voxel tree modeling for estimating leaf area density and woody material volume using 3-D LIDAR data — F. Hosoi, Y. Nakai and K. Omasa, 2013, ISPRS Annals of the Photogrammetry, Remote Sensing and Spatial Information Sciences II-5/W2, 115–120.
DOI: 10.5194/isprsannals-II-5-W2-115-2013 · Article page

The article combines airborne and ground-based LiDAR for LAD estimates. For woody volume, it describes a voxel solid model made of:

“consecutive voxels that filled the outer surface and the interior of the stem and large branches.”

Woody volume is then calculated by counting the corresponding voxels and multiplying by per-voxel volume. This is a published example of separating leaf-area/density estimates from solid woody volume rather than treating every crown voxel as solid wood.

Coarse geospatial voxel trees: VoxCity
VoxCity: A Seamless Framework for Open Geospatial Data Integration, Grid-Based Semantic 3D City Model Generation, and Urban Environment Simulation — Kunihiko Fujiwara, Ryuta Tsurumi, Tomoki Kiyono, Zicheng Fan, Xiucheng Liang, Binyu Lei, Winston Yap, Koichi Ito and Filip Biljecki.
Read arXiv manuscript, version 1 (2025) · VoxCity code repository

The manuscript describes a generator that downloads building heights, tree canopy heights, land cover and terrain data, then voxelizes and integrates them into a semantic voxel-city model. It is relevant as an example of coarse, geospatial tree-canopy voxels, not as a detailed branch/crown reconstruction method. Its input canopy-height data and voxelization should not be mistaken for an empirically validated subvoxel tree shape or porosity model.

Bibliographic caveat: the arXiv manuscript identifies Computers, Environment and Urban Systems as the journal, but I could not verify the final publication’s DOI and issue/article metadata from the pages fetched.

3. What a crown voxel actually contains: porosity and leaf-area density
Measured volume-based crown void fractions
A reinterpretation of the gap fraction of tree crowns from the perspectives of computer graphics and porous media theory — Yunfeng Zhu, Dongni Li, Jiangchuan Fan, Huaiqing Zhang, Markus P. Eichhorn, Xiangjun Wang and Ting Yun, 2023, Frontiers in Plant Science 14, article 1109443.
DOI: 10.3389/fpls.2023.1109443 · Full text at PMC

This study reconstructs leaves and branches from terrestrial laser scans, estimates leaf volumes with hexagonal-prism envelopes and branch segments with cylinders, then defines crown volume-based gap fraction as:

“one minus the ratio of the total plant leaf and branch volume to the canopy volume.”

The measured values for five individual trees were:

Sample tree	Crown volume (m³)	Equivalent leaf volume (m³)	Wood volume (m³)	Volume-based gap fraction	Equivalent leaf + wood fraction
Crepe myrtle	2.15	0.018	0.014	0.985	1.5%
Sakura	5.34	0.186	0.063	0.953	4.7%
Magnolia	2.18	0.051	0.019	0.968	3.2%
Rubber tree	86.17	1.71	0.560	0.973	2.6%
Ginkgo	14.86	0.52	0.101	0.958	4.2%
Thus, for these five trees under this study’s volume definition, the crown’s equivalent leaf-and-wood volume was about 1.5–4.7%, and its volume-based void fraction was about 95.3–98.5%. The authors also estimated a plantation-plot volume-based gap fraction of 0.970.

The paper separately reports voxel-scale gap fractions of 0.932 for crepe myrtle, 0.903 for sakura and 0.965 for magnolia. These are computed from the share of crown volume not occupied by voxels containing scanned points. They are voxel-occupancy measures, not the literal solid-volume fractions in the table; their values depend on grid size and point inclusion.

Important interpretation: these are not universal “wood-versus-air” constants. They are estimates from a particular method and sample of species. In particular, the study’s “equivalent” leaf volumes use an effective thickness intended to account for leaf curling and drooping; they are not simply the physical volume of leaf tissue.

Leaf-area density: an area-per-volume quantity, not a solid fraction
Retrieval of the leaf area density of Magnolia woody canopy with terrestrial Laser-scanning data — Wang Hongshu, Li Shihua, Guo Jiawei and Liang Zuqin, Journal of Remote Sensing.
DOI: 10.11834/jrs.20165061 · Article page

The retrieved page reports a maximum LAD of 1 m²/m³ at 4 m height and a calculated LAI of 3.20 m²/m², with a 1.26% relative error compared with LAI-2200 field measurements. The page notes that LAD’s direct accuracy could not be evaluated from LAI alone, because LAI does not preserve the vertical-layer information of LAD.

This is useful for the kind of value a subvoxel can carry: LAD indicates leaf area per crown volume. It is not a volume fraction. A high LAD does not mean a voxel is mostly solid.

Practical representation for an 8 m grid
An 8 m cubic cell is 512 m³. A reasonable representation should avoid assigning it a simple “solid tree” or “empty” label.

At the 8 m level: store tree/canopy presence, species or crown class if known, canopy height, and aggregate attributes such as average LAD or foliage/wood fraction.
Below that level: represent the crown envelope with fractional occupancy or a set of crown lobes, and retain the trunk/primary limbs as a skeleton or tapered segments if they matter at the viewing distance.
For physical occupancy: keep distinct fields for (a) crown-space occupancy, (b) leaf-area density, and (c) woody volume fraction. They answer different questions.
For rendering: use crown shape and major branch proportions as the stable structure; use foliage density/clumps for surface appearance; progressively simplify fine branches with distance.
For a first implementation, 0.5–1 m subvoxels are a defensible engineering starting range for testing crown shape inside an 8 m voxel (8–16 subdivisions per side). VoxR demonstrates tree-analysis voxel sizes in this range, but those studies do not prove that 0.5 m or 1 m is a perceptual threshold. Compare rendered results at the actual viewing distances and evaluate crown silhouette, height/width ratio and major branch structure.

As an illustration only, if an entire 512 m³ coarse cell were filled by crown space at the measured densities in Zhu et al., it would contain roughly 7.6–23.9 m³ of equivalent leaf-plus-wood volume, with the remainder classified as void by that study’s method. A real coarse voxel may contain only part of a crown or several trees, so use measured/modelled local crown volume and occupancy rather than applying those figures as universal constants.

Limits of what I could verify
I found no published universal threshold for how many branch levels, crown lobes, or subvoxels make a tree recognizable on an 8 m grid.
The measured 95.3–98.5% void values come from five trees and a particular geometric definition; they should be treated as a measured range in that study, not a universal tree constant.
The Magnolia LAD result provides a concrete measured value, but it is one study and does not establish a general LAD range for all tree species or seasons.
VoxCity is a useful coarse voxel-city reference, but I could not verify final journal DOI/issue metadata or a claim that it represents internal tree porosity.
I found the Purdue thesis Botanical Tree Perceived Realism Metric by David Hrusa (2021), but its PDF did not yield readable text, so I have not used it for specific findings or numbers.
Key sources
Runions, Lane & Prusinkiewicz (2007), Modeling Trees with a Space Colonization Algorithm, DOI 10.2312/NPH/NPH07/063-070.
Livny et al. (2010), Automatic Reconstruction of Tree Skeletal Structures from Point Clouds, DOI 10.1145/1866158.1866177.
Livny et al. (2011), Texture-lobes for tree modelling, DOI 10.1145/1964921.1964948.
Zhang et al. (2017), Tree Branch Level of Detail Models for Forest Navigation, DOI 10.1111/cgf.13088.
Polášek et al. (2021), ICTree: Automatic Perceptual Metrics for Tree Models, DOI 10.1145/3478513.3480519.
Hosoi, Nakai & Omasa (2013), Voxel tree modeling for estimating leaf area density and woody material volume using 3-D LIDAR data, DOI 10.5194/isprsannals-II-5-W2-115-2013.
Lecigne, Delagrange & Messier (2018), Exploring trees in three dimensions: VoxR…, DOI 10.1093/aob/mcx095.
Zhu et al. (2023), A reinterpretation of the gap fraction of tree crowns…, DOI 10.3389/fpls.2023.1109443.
Wang et al., Retrieval of the leaf area density of Magnolia woody canopy…, DOI 10.11834/jrs.20165061.
Fujiwara et al., VoxCity… arXiv manuscript.
## Where the tree stands, 2026-10-25: a design, not a body

**Corrected.** An earlier version of this section claimed the tree was already a being
with an identity and an origin, a `voxel-shapers::gaia` that answered *which tree is in
this cell*, and a `crates/shploingus/src/forest.rs` that built one body a frame. **None
of those files exist.** The tree in the game is still the block tree `flora.rs` writes:
one small trunk with a cross of leaves, re-derived wherever its cell is meshed. The
claim was written ahead of the code and is withdrawn here rather than left standing.

**What is true.** `voxel_mesh::Mesh` carries an `origin` and `Mesh::placed_at`, so a
body can be drawn at its own corner and may span a chunk boundary without claiming to
lie in one. That is the one piece the drawing half needs and it is landed.

**What the tree mob will be drawn with.** `docs/SHAPER-PROFILES.md` designs the 1/32
profile shaper: an analytic cross-section sampled thirty-two times per block axis
(31.25 mm), with a bark flute, a foliage void channel, an exact surface normal, and a
stage in 1/1024ths that scales the whole body so growth animates as a blend of two
stages rather than a rebuild. `voxel_model::profile32` implements that law and is
tested; **it has no pixels yet** — the mesh carrier and the shader arm are owed and are
named in that document's last section.

**What still must not be claimed.** A tree casts no shadow and does not shade the
ground it stands on: the horizon march and the shadow map see terrain only. No tree has
mass, because a profile is an envelope and not biomass (GAIA §5.1). And there is one
block tree per cell until the organ graph exists — the forest is a forest of the same
drawing, not a stand of beings.
