/**
 * Evaluates a simple boolean INI indicator expression against realtime +
 * constant values.
 *
 * Indicator expressions come from INI `bits`/indicator definitions and look
 * like `rpm > 2000 && map < 90` or just `launchControl`. This function turns
 * such an expression into a boolean so the UI can light up an indicator.
 *
 * # Evaluation strategy & security
 *
 * Expressions are evaluated with `new Function(...)`. To keep that safe, the
 * expression is **never** passed through as-is: it is first tokenized by a
 * restrictive regex that admits ONLY:
 *   - identifiers (`[a-zA-Z_][a-zA-Z0-9_]*`),
 *   - numeric literals (`[0-9.]+`),
 *   - logical operators (`&&`, `||`),
 *   - comparison/arithmetic operator runs (`[<>=!&]+`),
 *   - parentheses.
 * Anything else (strings, semicolons, function-call syntax, `{...}`, etc.) is
 * dropped by the tokenizer and therefore can never reach `new Function`.
 *
 * Each surviving identifier is then substituted with its numeric value
 * (constants override defaults; realtime data overrides constants; an unknown
 * identifier becomes `0`) *before* the string is compiled. So the only things
 * `new Function` ever sees are numbers, operators, and parens — no attacker-
 * controllable identifiers survive as code. This is why the `Function` path is
 * acceptable here despite the usual eval-injection concerns.
 *
 * # Fast path
 *
 * An expression that is a single bare identifier (e.g. `checkEngine` or
 * `launchControl`) short-circuits: it is treated as truthy iff the resolved
 * value is defined and non-zero. This avoids compiling a `Function` for the
 * common case.
 *
 * # Fallback semantics
 *
 * - An unknown identifier resolves to `0` (so `missing > -1` is true, but
 *   `missing > 0` is false — matching TunerStudio's behaviour for undefined).
 * - ANY error during tokenize/build/eval returns `false` (fail-safe: an
 *   unparseable indicator expression lights nothing up rather than throwing).
 */
export function evaluateIndicatorExpression(
  expression: string,
  data: Record<string, number>,
  constants: Record<string, number> = {},
): boolean {
  // Realtime data takes precedence over constants (spreads later override
  // earlier keys), so a name defined in both resolves to the live value.
  const context = { ...constants, ...data };

  try {
    const expr = expression.trim();

    // Fast path: a single bare identifier is truthy iff defined and non-zero.
    // Covers the common `indicatorName`-only expressions without compiling.
    if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(expr)) {
      const val = context[expr];
      return val !== undefined && val !== 0;
    }

    // Tokenize with a restrictive accept set (see security note above). Any
    // character/class not listed here is silently dropped, which is what keeps
    // the subsequent `new Function` safe from injection.
    const tokens = expr.match(/([a-zA-Z_][a-zA-Z0-9_]*|[0-9.]+|&&|\|\||[<>=!&]+|[()]+)/g);
    if (!tokens) return false;

    // Rebuild the expression as a numeric-only string: every identifier is
    // replaced by its resolved value (or 0 if unknown). After this loop, no
    // alphabetic identifier remains in `result` — only numbers/operators.
    let result = '';
    for (const token of tokens) {
      if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(token)) {
        const val = context[token];
        result += val !== undefined ? val : 0;
      } else {
        result += token;
      }
    }

    // `result` now contains only numbers + operators, so `new Function` is
    // constrained to arithmetic/comparison — it cannot reference identifiers.
    const evalFn = new Function('return (' + result + ') ? true : false');
    return evalFn();
  } catch {
    // Fail-safe: any error (bad expression, odd operator sequence, etc.)
    // yields false rather than propagating to the indicator render path.
    return false;
  }
}

/**
 * Collects every identifier referenced across one or more indicator
 * expressions. Used to know which realtime channels/constants must be
 * available before the expressions can be evaluated (e.g. to subscribe or to
 * surface a list of required channels). Purely lexical — does not evaluate.
 */
export function extractExpressionVariables(expressions: string[]): string[] {
  const vars = new Set<string>();
  for (const expression of expressions) {
    const tokens = expression.match(/[a-zA-Z_][a-zA-Z0-9_]*/g);
    tokens?.forEach((token) => vars.add(token));
  }
  return Array.from(vars);
}
