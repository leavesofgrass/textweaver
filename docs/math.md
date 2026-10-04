# Reading and writing math

textweaver reads math aloud in plain English, and it turns math in your Markdown into MathML, the web's own math format, so screen readers can read it. You write formulas in LaTeX, such as `$x^2$`, or in ASCIIMath. When textweaver reads, it says "x squared", not "dollar x caret 2 dollar", and it highlights the part of the formula each word comes from. When `tw convert` makes a web page, each formula becomes real MathML, with the LaTeX kept inside it so you can still copy it. This guide is for students and teachers who read or write math by ear, with or without a screen reader.

This guide is written to be read with a screen reader. Each task section starts with the command, then explains it.

## Before you start

- Install textweaver. The [installation guide](install.md) explains how.
- Have a document with math in it. A Markdown file (`.md`) or a plain text file (`.txt`) works best.
- Math reading is on by default. You do not need to change anything to hear math.
- To change how math is spoken, you edit your settings file, `settings.toml`, in a text editor. The section "Choose how math is spoken" below shows how to find it.

## Hear the math in a document

```powershell
tw open "Calculus notes.md"
```

This opens the document in the terminal reader. Read it the way you read anything else; the [reading guide](reading.md) explains the keys. When the reading reaches a formula, you hear the formula in words. For example, the sentence "The area is `$\pi r^2$`." is read as "The area is pi r squared."

The dollar signs and other delimiters are never spoken. The words around the formula are read as usual.

The `textweaver` command opens the same terminal reader:

```powershell
textweaver "Calculus notes.md"
```

## How textweaver finds math

textweaver only reads something as math when it is marked as math. Text that is not marked stays exactly as written. So `snake_case` in a sentence is never "snake sub case", and prices such as "$5 and $10" stay prices.

These marks are recognized:

- `$…$`: inline math, inside a sentence. For example, `$x^2$`.
- `$$…$$`: display math, a formula on its own. For example, `$$\frac{a}{b}$$`.
- `\(…\)`: inline math, the LaTeX way. For example, `\(a+b\)`.
- `\[…\]`: display math, the LaTeX way. For example, `\[x^2\]`.
- ASCIIMath between two copies of a character you choose, usually a backtick. For example, `` `x^2/2` ``. This one is off until you turn it on (see "Choose how math is spoken" below).

A few rules apply to every mark:

- A formula never crosses a blank line. If the closing mark is in the next paragraph, nothing is math.
- An empty formula, or one that is only spaces, is not math.
- A backslash before a dollar sign, `\$`, is a real dollar sign. It never starts or ends a formula.
- ASCIIMath must open and close on the same line.

### Why prices are not math

A single dollar sign follows the same rule as Pandoc, a popular document converter. All three of these must be true for `$…$` to be math:

- The character right after the opening `$` is not a space, a tab, or a line break.
- The character right before the closing `$` is not a space, a tab, or a line break.
- The character right after the closing `$` is not a digit (0 to 9).

Only the first `$` after the opening one can close it. If that one breaks a rule, the opening `$` is just a dollar sign.

So these are all read as prices, not math:

- "It costs $5 and $10." The second `$` is followed by the digit 1.
- "$5-$10". The second `$` is followed by the digit 1.
- "from $3.99 to $4.50 each". The second `$` is followed by the digit 4.
- "US$5 or US$6". The second `$` is followed by the digit 6.
- "price: $ 5 $". There is a space after the opening `$`.

And these are math: `$x$`, `$5$`, and `$2x = 4$`. The last one is read as "2 x equals 4".

When a formula touches a word, textweaver keeps them apart. "the `$n$`th term" is read as "the n th term".

## What you hear

textweaver speaks math in natural English, in the style of ClearSpeak, a way of speaking math designed for students. Digits and letters are read as themselves. Structure, such as a fraction or a power, becomes words.

Some examples at the default, normal, verbosity:

- `\frac{a}{b}` is "a over b".
- `\frac{3}{4}` is "3 fourths".
- `x^2` is "x squared", and `y^{3}` is "y cubed".
- `x_i` is "x sub i".
- `\sqrt{x}` is "square root of x".
- `\sqrt[3]{8}` is "cube root of 8".
- `a \times b \leq c` is "a times b less than or equal to c".
- `|x - 1|` is "the absolute value of x minus 1".
- `\sum_{i=1}^{n} i` is "the sum from i equals 1 to n of i".
- `\int_0^1 x^2 \, dx` is "the integral from 0 to 1 of x squared d x".
- `f(x) = x^2 - 2x + 1` is "f of x equals x squared minus 2 x plus 1".
- `x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` is "x equals the fraction with numerator negative b plus or minus square root of b squared minus 4 a c and denominator 2 a".
- `\begin{pmatrix} a & b \\ c & d \end{pmatrix}` is "the 2 by 2 matrix; row 1: a, b; row 2: c, d".

Words such as "end fraction", "end root", and "end exponent" tell you where a part stops. At low and normal verbosity you hear them only when a part is not simple and something follows it. So "x plus 1 over 2" is never ambiguous.

A few symbols are also spoken when they appear in ordinary text, outside any formula, because some speech engines skip them: `×` is "times", `÷` is "divided by", `≤` is "less than or equal to", `≥` is "greater than or equal to", `≈` is "approximately equal to", `≠` is "not equal to", `∞` is "infinity", `→` is "approaches", and `←` is "from".

### Low, normal, and high verbosity

There are three levels. Low is short. Normal is the default and follows ClearSpeak. High adds "the", end markers, and names such as "cap" for capital letters, so you could write the formula back down exactly.

The same formulas at each level:

- `\frac{x+1}{2} + 3`
  - Low: "fraction x plus 1 over 2 end fraction plus 3"
  - Normal: "the fraction with numerator x plus 1 and denominator 2 end fraction plus 3"
  - High: "the fraction with numerator x plus 1 and denominator 2 end fraction plus 3"
- `x^{n+1}`
  - Low: "x to the n plus 1"
  - Normal: "x raised to the n plus 1 power"
  - High: "x raised to the exponent n plus 1 end exponent"
- `\frac{a}{b} + x^2`
  - Low: "a over b plus x squared"
  - Normal: "a over b plus x squared"
  - High: "the fraction a over b end fraction plus x squared"
- `\sqrt{x}`
  - Low: "square root of x"
  - Normal: "square root of x"
  - High: "the square root of x end root"
- `\Gamma(n) = (n-1)!`
  - Low: "gamma n equals open paren n minus 1 close paren factorial"
  - Normal: "capital gamma n equals open paren n minus 1 close paren factorial"
  - High: "capital gamma open paren n close paren equals open paren n minus 1 close paren factorial"

## Choose how math is spoken

```powershell
tw settings path
```

This prints where your settings are. The line that starts with "Settings file:" gives the full path of `settings.toml`. If the file does not exist yet, the line ends with "not created yet; every setting is at its default". You can create it yourself, as a plain text file at that path.

Open `settings.toml` in any text editor. Add a `[normalization]` section, or add these lines to the one that is there:

```toml
[normalization]
math = true
math_verbosity = "low"
asciimath_delimiter = "`"
```

The three math settings:

- `math`: read math as math. `true` or `false`. The default is `true`. With `false`, formulas are left as written, and the other reading rules see them as ordinary text. For example, `$x^2$` is then read as "$x caret 2$", and a dollar sign followed by a digit is read as money.
- `math_verbosity`: how much is said. `"low"`, `"normal"`, or `"high"`, in quotes. The default is `"normal"`.
- `asciimath_delimiter`: the one character written on both sides of ASCIIMath, in quotes. A backtick, `` "`" ``, is the usual choice. It is not set by default, so no ASCIIMath is read, because in Markdown a backtick marks code.

Save the file. The next time you start textweaver, it uses the new values. You only need the lines you change; anything you leave out keeps its default.

If a value is wrong, for example `math_verbosity = "loud"`, only that setting falls back to its default, and textweaver tells you: "Some settings were invalid and use their defaults: normalization.math_verbosity has an invalid value".

Where these settings apply:

- The terminal reader (`textweaver` and `tw open`), the GUI, `tw speak`, and `tw export-audio` all follow the three settings, because they share the same reading and speech code.

## Hear math with MathCAT

MathCAT is the math engine NVDA and JAWS use. textweaver can speak math with it instead of its own wording, in two styles:

- ClearSpeak: the style of the ClearSpeak rules used in classrooms and on tests. For example, `\sqrt{x}` is "the square root of x".
- SimpleSpeak: shorter, with "end fraction" and "end root" where the structure needs them.

MathCAT is in builds made with the `mathcat` feature (see "Build it" below). To use it, add this to `settings.toml`, in the `[reading]` section:

```toml
[reading]
math_engine = "mathcat"
```

The values:

- `"builtin"`: textweaver's own math speech. This is the default.
- `"mathcat"`: MathCAT in ClearSpeak.
- `"mathcat_simplespeak"`: MathCAT in SimpleSpeak.

You can also change it in the settings screen (`Shift+F10`): "Math speech". The change applies from the next sentence read.

What stays the same:

- `math_verbosity` still sets how much is said. Low is MathCAT's "Terse", normal is "Medium", and high is "Verbose", which adds end words such as "end fraction".
- Math is found the same way, so prices are still not math.
- A formula MathCAT cannot read is read by textweaver's own speech.

What is different:

- MathCAT reads the whole formula at once, so the highlight covers the whole formula while you hear any part of it.
- MathCAT speaks in the document's language when it has rules for it: German, Greek, English, Spanish, Finnish, French, Hungarian, Indonesian, Norwegian (Bokmål), Polish, Russian, Swedish, Vietnamese, and Chinese. Any other language is read in English. textweaver's own math speech is English only.
- MathCAT spells the letter "a" as "eigh", so the speech engine says the letter and not the word "a".
- With `punctuation = "all"`, MathCAT's pauses are left out, so you do not hear "comma" inside a formula.

Some examples in ClearSpeak at normal verbosity, with the words exactly as MathCAT sends them to the speech engine:

- `\frac{a}{b}` is "eigh over b".
- `x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` is "x is equal to; the fraction with numerator; negative b plus or minus; the square root of b squared minus 4 eigh c; and denominator 2 eigh".
- `\sin^2 x + \cos^2 x = 1` is "sine squared of x, plus cosine squared of x; is equal to 1".

To compare the two engines, open `fixtures/c1/quadratic.md` and read it with each setting.

### Build it

```powershell
cargo build --release -p textweaver-tui --features mathcat
```

This builds the reader with MathCAT. MathCAT's rules are built into the program, so nothing is downloaded when it runs. Without the feature, `math_engine = "mathcat"` changes nothing and math is read by textweaver's own speech.

## Check what textweaver will say

```powershell
tw speak --backend null --json 'The area is $x^2$.'
```

This shows the words textweaver would send to the speech engine, without making any sound. `--backend null` is the silent speech engine. `--json` prints the spoken text and its map back to your text.

Put the text in single quotes. In PowerShell and in bash, single quotes stop the shell from treating `$x` as a variable.

Here is part of what it printed, trimmed:

```text
"text": "The area is x squared.",
"kind": "text",
"offset_map": {
  "spans": [
    { "spoken": { "start": 0, "end": 12 }, "source": { "start": 0, "end": 12 }, "kind": "literal" },
    { "spoken": { "start": 12, "end": 12 }, "source": { "start": 12, "end": 13 }, "kind": "elided" },
    { "spoken": { "start": 12, "end": 13 }, "source": { "start": 13, "end": 14 }, "kind": "literal" },
    { "spoken": { "start": 14, "end": 21 }, "source": { "start": 14, "end": 16 }, "kind": "expanded" },
    { "spoken": { "start": 21, "end": 21 }, "source": { "start": 16, "end": 17 }, "kind": "elided" },
    ...
```

How to read it:

- `text` is what is spoken: "The area is x squared."
- Each span links spoken text to your text. Spoken positions count bytes; source positions count characters.
- The two `elided` spans are the dollar signs. They are in your text but not spoken.
- The `expanded` span says that the word "squared" (spoken bytes 14 to 21) stands for `^2` (source characters 14 to 16).

`tw speak` uses your `math_verbosity`, so the JSON above follows your setting. You can also check it with `tw export-audio` and the `recording` speech engine, a test engine that plays nothing and writes a placeholder audio file:

```powershell
tw export-audio sample.txt --out sample.wav --backend recording --json
```

In the printed JSON, each sentence has a `text` (your text) and a `spoken` (what is said). With `sample.txt` holding the sentence "Half `$\frac{a}{b} + x^2$` done." and `math_verbosity = "high"`, it printed:

```text
"text": "Half $\\frac{a}{b} + x^2$ done.",
"spoken": "Half the fraction a over b end fraction plus x squared done.",
```

At `"low"` and `"normal"`, the same sentence printed "Half a over b plus x squared done." You can delete `sample.wav` afterwards.

## Follow the highlight inside a formula

While textweaver reads a formula, the highlight moves through it word by word, just as it does in ordinary text. Each spoken word highlights the part of the formula it stands for:

- "x" highlights the `x`.
- "squared" highlights `^2`.
- "plus" highlights `+`.
- In `\frac{a}{b}`, "a" highlights `a`, "over" highlights `}{`, the brace pair between the top and the bottom, and "b" highlights `b`.
- "end fraction" highlights the closing brace `}` of the bottom part.
- "alpha" highlights `\alpha`, and "less than or equal to" highlights `\leq`.

The dollar signs and other delimiters are never highlighted on their own, because they are never spoken.

Some words have no part of the formula of their own. "power" in "x raised to the n plus 1 power" is one. Such a word has nothing of its own to highlight.

## See math as Unicode

The reading view shows math as it is written, `$x^2$`. To see it drawn instead, set `math_display = "unicode"` under `[reading]` in `settings.toml`, or choose "Math on screen" in the settings list. Each formula is then shown on one line in Unicode characters, as star did:

- scripts become raised or lowered characters where Unicode has them: `x²`, `aᵢ`, `x₁₀`. Where it has none, the script is written out: `x^(1⁄y)`;
- fractions use the fraction slash: `1⁄2`, and `(a + b)⁄c` for longer parts;
- roots use the root signs: `√2`, `∛8`;
- `\mathbb{R}` and the other math fonts use their Unicode letters: `ℝ`.

Only the screen changes. textweaver still reads the formula from its source, the highlight and the cursor stay on the formula, and edit mode and exploring a formula show the source. A screen reader reading the screen hears the Unicode characters, which some voices say well and some do not, and a Braille display shows them as its table allows; the source is often clearer there. Try it on `fixtures/g/math.md`.

## Turn math into MathML for a web page

```powershell
tw convert notes.md --to html
```

This writes `notes.html` next to `notes.md`. Each formula between dollar signs becomes MathML: `$x^2$` inside a sentence and `$$ … $$` as a formula on its own. The [converting guide](converting.md) explains `tw convert` in full.

What the MathML contains:

- The formula's real structure: a fraction is a fraction, a power is a power.
- Your LaTeX as the formula's text alternative (the `alttext` attribute), for tools that cannot read MathML.
- Your LaTeX again as an annotation, so copying the formula can still give you the LaTeX.
- For display math, `display="block"`, so the formula stands on its own line.

A formula textweaver cannot read completely, such as `$\foo{x}$` with an unknown command, is not turned into a half-right formula. It is shown as its LaTeX source, in code style.

Things to know:

- Only dollar signs make math in `tw convert`. `\(…\)` and `\[…\]` are not turned into MathML. In Markdown, a backslash before a bracket only means "this is a plain bracket", so `\(a+b\)` becomes "(a+b)".
- With `--flavor commonmark`, there is no math at all: dollar signs stay as dollar signs.
- A code block that starts with three backticks and the word `asciimath` (or `am`) becomes one display formula, written in ASCIIMath.

### Keep math as LaTeX

```powershell
tw convert notes.md --to html --no-math
```

`--no-math` leaves every formula as its LaTeX source, with its dollar signs. Code blocks marked `asciimath` stay code blocks.

### Read ASCIIMath in code spans

```powershell
tw convert notes.md --to html --asciimath
```

Course material written for MathJax often puts ASCIIMath between single backticks, like `` `x^2/2` ``. `--asciimath` turns every inline code span into a formula. Use it only for documents where code spans hold math, because real code in backticks would also become a formula.

### EPUB, Word, braille, and PDF

```powershell
tw convert notes.md --to epub
```

These outputs typeset math from Markdown too, and never print the dollar signs:

- EPUB: MathML, as in HTML, with the LaTeX as its text alternative.
- Word (`docx`): Word's own equations (Office Math), with real fractions, scripts, roots, and matrices. Word draws them and can read them aloud.
- PDF: the formula in print form, such as πr² or (a + b)/2, tagged as a formula whose text alternative is how it is read aloud, for example "pi r squared".
- Braille (`brf`): math braille, in Nemeth by default or in UEB mathematics, in a build with MathCAT (see "Math in braille files" below). In a build without it, the formula as it is read aloud, for example "pi r squared".

Plain text (`txt`) keeps the LaTeX source with its dollar signs, as textweaver reads it.

### Math in braille files

In a build with MathCAT (see "Build it" above; `tw` needs `--features mathcat` too), braille files show math in a math braille code:

```powershell
tw convert notes.md --to brf
tw convert notes.md --to brf --math-code ueb
```

- **Nemeth** (the default): the Nemeth Code, inside the Unified English Braille text around it. Each formula starts with the opening Nemeth indicator and a space, and ends with a space and the Nemeth terminator, as BANA's guidance for Nemeth in UEB contexts says. A number on its own, such as the 2 in "2 roots", stays in UEB.
- **UEB**: Unified English Braille's own mathematics, with no switch indicators.

After a braille file is written, `tw convert` says which code it used, for example "Math braille: Nemeth." Exports from the reader (`export brf`) use `math_code` in the `[braille]` section of `settings.toml`:

```toml
[braille]
math_code = "ueb"
```

Lines are 40 cells. A long formula moves to the next line before a comparison sign, such as the equals sign, or before a plus or minus sign, and an indicator always stays on the line with what it belongs to. Try `fixtures/c4/quadratic.md`; `fixtures/c4/quadratic.nemeth.brf` and `quadratic.ueb.brf` are what it should become.

A formula the parser had to repair, or one MathCAT cannot write, is written as its spoken words, in the same braille as the text around it. The summary says so once, for example "1 formula is in spoken words: MathCAT could not write it in Nemeth braille." In a build without MathCAT, every formula is written that way, and the summary says the build has no math braille.

## Read MathML with your screen reader

Open the HTML page in a web browser. How you hear the math depends on your screen reader:

- NVDA (Windows): install the free MathCAT add-on. MathCAT reads MathML aloud and lets you explore a formula part by part.
- JAWS (Windows): math support is built in. JAWS reads MathML in web browsers and lets you explore a formula in its math viewer.
- VoiceOver (macOS and iOS): math support is built in. VoiceOver reads MathML in Safari and lets you move through a formula part by part.

A screen reader that does not understand MathML may read the text alternative instead, which is your LaTeX source.

textweaver reads MathML (`<math>`) as math in every web page it opens, HTML or MHTML, the same as in EPUB books: a formula becomes LaTeX in the text, as in a Markdown file, and is spoken as math, by MathCAT if you chose it. The book's or page's own TeX is used when the formula carries it. A formula with no math inside, only a text alternative (`alttext`), is read as that text; an `epub:switch` in an EPUB book is read once, as its first case that holds MathML, else its default.

## Write math in Markdown

Put inline math between single dollar signs, with no space inside the dollar signs: `$x^2$`, not `$ x^2 $`. Put a formula on its own between double dollar signs, `$$ … $$`, on its own line.

Dollar signs are the best choice in Markdown. They work for reading aloud and for `tw convert`. In a Markdown file, `\(…\)` and `\[…\]` do not work, because Markdown uses the backslash for its own purpose. They do work in plain text files.

### LaTeX cheat sheet

Each item shows what to type and, where it was checked, what normal verbosity says.

- Fraction: `\frac{a}{b}`, "a over b". Also `\dfrac` and `\tfrac`.
- Fraction with more on top: `\frac{x+1}{x-1}`, "the fraction with numerator x plus 1 and denominator x minus 1".
- Power: `x^2`, "x squared". Put a longer power in braces: `x^{n+1}`, "x raised to the n plus 1 power".
- Subscript: `x_i`, "x sub i". Longer: `x_{ij}`, "x sub i j".
- Subscript and power together: `x_i^2`, "x sub i squared".
- Square root: `\sqrt{x}`, "square root of x".
- Other roots: `\sqrt[3]{8}`, "cube root of 8", and `\sqrt[n]{x}`, "n-th root of x".
- Greek letters: `\alpha`, `\beta`, `\pi`, `\theta`, `\lambda`, `\mu`, `\sigma`. `\alpha + \beta` is "alpha plus beta". A capital letter starts with a capital: `\Gamma`, "capital gamma", and `\Delta`.
- Sum: `\sum_{i=1}^{n} i`, "the sum from i equals 1 to n of i".
- Product: `\prod_{i=1}^{n}`.
- Integral: `\int_0^1 x^2 \, dx`, "the integral from 0 to 1 of x squared d x". `\,` is a small space. Also `\iint` and `\oint`.
- Limit: `\lim_{x \to 0}`, "the limit as x approaches 0 of".
- Plus or minus: `\pm`, "plus or minus".
- Times and dot: `\times`, "times", and `\cdot`.
- Division: `\div`.
- Comparisons: `\leq` (or `\le`), "less than or equal to", `\geq`, `\neq`, and `\approx`.
- Infinity: `\infty`.
- Derivatives: `\frac{d}{dx}`, "d over d x", and `\partial`, "partial".
- Choose: `\binom{n}{k}`, "n choose k".
- Absolute value: `|x - 1|`, "the absolute value of x minus 1".
- Words inside math: `\text{speed} = \frac{d}{t}`, "speed equals d over t".
- Sets of numbers: `\mathbb{R}`, "the real numbers".
- Accents: `\hat{x}`, "x hat", `\bar{x}`, "x bar", and `\vec{v}`, "vector v".
- Big brackets: `\left( \frac{a}{b} \right)^2`, "open paren a over b close paren squared".
- Matrix: `\begin{pmatrix} a & b \\ c & d \end{pmatrix}`, "the 2 by 2 matrix; row 1: a, b; row 2: c, d". `&` separates columns and `\\` ends a row. `bmatrix` works the same way, and `vmatrix` is read as a determinant.
- Cases: `\begin{cases} x & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}`.

A command textweaver does not know is spoken by its name. `\foo{x}` is read as "foo x".

### ASCIIMath cheat sheet

ASCIIMath is shorter to type and needs no backslashes. In the reader, it is only read as math when you set `asciimath_delimiter` and put the delimiter on both sides.

- Fraction: `a/b`. `x^2/2` is "the fraction with numerator x squared and denominator 2".
- Fraction with more on top or bottom: use brackets, `(x+1)/(x-1)`.
- Power: `x^2`. Longer: `x^(n+1)`.
- Subscript: `x_i`. Longer: `x_(n+1)`, "x sub n plus 1".
- Square root: `sqrt(x)`. `sqrt(x+1)/(x-1)` is "the fraction with numerator square root of x plus 1 and denominator x minus 1".
- Other roots: `root(3)(x)`, "cube root of x".
- Greek letters: `alpha`, `beta`, `pi`, `theta`, and capitals such as `Gamma`.
- Sum: `sum_(i=1)^n i^3`.
- Product: `prod_(i=1)^n`.
- Integral: `int_0^1 f(x) dx`, "the integral from 0 to 1 of f of x d x".
- Limit: `lim_(x->oo) 1/x = 0`, "the limit as x approaches infinity of 1 over x equals 0". `->` is an arrow and `oo` is infinity.
- Plus or minus: `+-`. Times: `xx`. Dot: `*`.
- Comparisons: `<=`, `>=`, and `!=`.
- Absolute value: `abs(x-1)`, "the absolute value of x minus 1".
- Inverse functions: `sin^-1(x)`, "inverse sine of x".
- Words inside math: `"if" x > 0`, "if x greater than 0".
- Sets of numbers: `bbb R`, "the real numbers".
- Accents: `hat x + bar y`, "x hat plus y bar". Also `vec v`.
- Matrix: `[[a,b],[c,d]]`, "the 2 by 2 matrix; row 1: a, b; row 2: c, d".

For a whole ASCIIMath formula in Markdown, use a fenced code block marked `asciimath`:

````markdown
```asciimath
sum_(i=1)^n i^3=((n(n+1))/2)^2
```
````

`tw convert` turns it into one display formula. The reader treats it as a code block, so it is not read as math (see below).

## Exploring a formula part by part

Put the cursor on a formula and press **Alt+Shift+X** (or run `explore math` from the palette). You hear "Exploring math:" and the whole expression. Then:

- **Right** and **Left**: the next or previous term at this level.
- **Down**: into the part, such as a fraction's numerator, a superscript, or what is under a root. **Right** then moves to the denominator or the next part.
- **Up**: back out to the part around it.
- **Home** and **End**: the first and last term at this level.
- **Space** or **Enter**: say the part again.
- **Escape**: leave. Any other key leaves too.

Each step says the part's role and the part, such as "numerator, a plus b" or "superscript, 2", and highlights it on screen; the cursor moves to it, so a magnifier or screen reader follows. At an edge nothing moves and you hear "Last term.", "First term.", "No parts inside.", or "Whole expression." The wording follows `math_verbosity`, as above. It works on `$…$`, `$$…$$`, `\(…\)`, `\[…\]`, and ASCIIMath in backticks, and on Word equations, which textweaver reads as LaTeX.

### Exploring with MathCAT

With `math_engine = "mathcat"` or `"mathcat_simplespeak"`, in a build with MathCAT, **Alt+Shift+X** explores the formula with MathCAT's own navigation, the one NVDA's MathCAT add-on uses. The keys are the same:

- **Right** and **Left**: the next or previous part.
- **Down** and **Up**: zoom in to the parts inside, and back out.
- **Home** and **End**: the start and the end.
- **Space** or **Enter**: say the part again. **Escape** leaves.

Each step is said once, in MathCAT's words. The status line shows those words, then the braille code and the braille of the part you reached: in a fraction's numerator a + b, the words end with `Nemeth: ⠁⠬⠃`. With `cursor = "status"` in `[accessibility]`, a Braille display follows the status line, so you read the part in `math_code`'s braille there. The braille is in Unicode braille cells; whether the display shows them as dots depends on your screen reader's braille table (see [Using textweaver with a screen reader](screen-readers.md)).

At an edge nothing moves: you hear the boundary sound and MathCAT says why. The highlight covers the whole formula while you explore, because MathCAT does not say where each part is in your text. A formula MathCAT cannot read is explored with textweaver's own navigator, as above. The built-in engine (`"builtin"`, the default) explores as before, without braille on the status line.

You can also explore a formula in your screen reader: convert the document to HTML and open it in a web browser (see "Read MathML with your screen reader" above).

## If something goes wrong

- A formula is read as symbols, such as "dollar x caret 2 dollar". Check the marks. There must be no space right after the opening `$` and no space right before the closing `$`, and the closing `$` must not be followed by a digit. Also check that `math` is not set to `false` in `[normalization]`.
- A price is read as math. This happens when a later dollar sign can close it, as in "Pay $5 and 6$ more", which is read as "Pay 5 a n d 6 more". Reword the sentence, or put a backslash before the first dollar sign: `\$5`. In a plain text file you may then hear the word "backslash". In a Markdown file, see the next item.
- In a Markdown file, `\$x\$` is read as the formula "x". This is a known problem: Markdown removes the backslashes before textweaver reads the text. `tw convert` shows it correctly as "$x$". In the reader, write the dollar signs so they break the rules above, for example with a space after the first one.
- `\(a+b\)` in a Markdown file is read as "(a plus b)", not as math. Markdown removes the backslash. Use `$a+b$` instead.
- ASCIIMath in backticks is read as symbols, such as "x caret 2 slash 2". In the reader, set `` asciimath_delimiter = "`" `` in `[normalization]`; it is off by default. In a Markdown file, this still does not work, because Markdown removes the backticks around code before textweaver reads the text. It works in plain text files. For `tw convert`, add `--asciimath`.
- A code block marked `asciimath` is read as "code block skipped". The reader skips code blocks by default (`[speech] skip_code`). With `skip_code = false` you hear the ASCIIMath source, not math.
- Your `math_verbosity` has no effect. Check the spelling: `"low"`, `"normal"`, or `"high"`, in quotes. A wrong value is reported and falls back to `"normal"`.
- A formula with a mistake, such as a missing brace, is still read as far as it makes sense. `\frac{1}{` is read as "the fraction with numerator 1 and denominator". In `tw convert` output it is shown as its LaTeX source instead of MathML.
- In the web page, a formula shows up as code. textweaver could not read it completely, so it kept your LaTeX. Look for an unknown command or a missing brace.
- In the web page, "$5-$10" or "US$5 or US$6" became a formula. The default Markdown engine uses its own rule for dollar signs, which is looser than the reader's. Convert with `--engine comrak`, which follows the same rule as the reader, or write the second price as `\$10`.

## See also

- [Converting documents](converting.md): everything `tw convert` can do, including HTML templates and other formats.
- [Reading and navigating](reading.md): the keys for reading a document aloud.
- [Settings](settings.md): where settings live, and how to export, import, and reset them.
- [Audio export](audio-export.md): `tw export-audio`, which reads a document with math into an audio file.
- [Using textweaver with a screen reader](screen-readers.md): JAWS, NVDA, VoiceOver, and Orca.
- [ADR-0018: Math](adr/0018-math.md): the design decision behind math reading and MathML.
- [ADR-0029: MathCAT speech](adr/0029-mathcat-speech.md): MathCAT as a second math engine.
- [ADR-0036: Math braille and navigation](adr/0036-math-braille-and-navigation.md): Nemeth and UEB in braille files, and exploring with MathCAT.
- [ADR-0005: Narration and the OffsetMap](adr/0005-narration-and-offset-map.md): how spoken words are mapped back to your text for highlighting.
- [Documentation index](README.md)
