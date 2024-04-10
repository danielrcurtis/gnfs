using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Numerics;
using System.Runtime.Serialization;

namespace ExtendedArithmetic
{
    [DataContract]
    public class Polynomial : ICloneable<Polynomial>, IComparable, IComparable<Polynomial>, IEquatable<Polynomial>
    {
        public static class Algorithms
        {
            public static BigInteger EulersCriterion(BigInteger a, BigInteger p)
            {
                BigInteger exponent = (p - 1) / 2;
                return BigInteger.ModPow(a, exponent, p);
            }

            public static int LegendreSymbol(BigInteger a, BigInteger p)
            {
                if (p < 2L)
                {
                    throw new ArgumentOutOfRangeException("p", $"Parameter 'p' must not be < 2, but you have supplied: {p}");
                }

                if (a == 0L)
                {
                    return 0;
                }

                if (a == 1L)
                {
                    return 1;
                }

                int num;
                if (a % 2 == 0L)
                {
                    num = LegendreSymbol(a / 2, p);
                    if (((p * p - 1) & 8) != 0L)
                    {
                        num = -num;
                    }
                }
                else
                {
                    num = LegendreSymbol(p % a, a);
                    if ((((a - 1) * (p - 1)) & 4) != 0L)
                    {
                        num = -num;
                    }
                }

                return num;
            }

            public static BigInteger LegendreSymbolSearch(BigInteger start, BigInteger modulus, BigInteger goal)
            {
                if (goal != -1L && goal != 0L && goal != 1L)
                {
                    throw new Exception(string.Format("Parameter '{0}' may only be -1, 0 or 1. It was {1}.", "goal", goal));
                }

                BigInteger bigInteger;
                for (bigInteger = start; !(LegendreSymbol(bigInteger, modulus) == goal); ++bigInteger)
                {
                }

                return bigInteger;
            }

            public static BigInteger TonelliShanks(BigInteger n, BigInteger p)
            {
                BigInteger bigInteger = LegendreSymbol(n, p);
                if (bigInteger != 1L)
                {
                    throw new ArithmeticException($"Parameter n is not a quadratic residue, mod p. Legendre symbol = {bigInteger}");
                }

                if (p.Mod(4) == 3L)
                {
                    return BigInteger.ModPow(n, (p + 1) / 4, p);
                }

                BigInteger bigInteger2 = p - 1;
                BigInteger bigInteger3 = 0;
                while (bigInteger2.Mod(2) == 0L)
                {
                    bigInteger2 /= (BigInteger)2;
                    ++bigInteger3;
                }

                if (bigInteger3 == 0L)
                {
                    throw new Exception();
                }

                if (bigInteger3 == 1L)
                {
                    throw new Exception("This case should have already been covered by the p mod 4 check above.");
                }

                BigInteger value = BigInteger.ModPow(LegendreSymbolSearch(0, p, -1), bigInteger2, p);
                BigInteger bigInteger4 = BigInteger.ModPow(n, (bigInteger2 + 1) / 2, p);
                BigInteger bigInteger5 = BigInteger.ModPow(n, bigInteger2, p);
                BigInteger bigInteger6 = 1;
                BigInteger bigInteger7 = bigInteger3;
                while (bigInteger5 != 1L && bigInteger6 < bigInteger7)
                {
                    BigInteger exponent = BigInteger.Pow(2, (int)(bigInteger7 - bigInteger6 - 1));
                    BigInteger bigInteger8 = BigInteger.ModPow(value, exponent, p);
                    bigInteger4 = BigInteger.Multiply(bigInteger4, bigInteger8).Mod(p);
                    bigInteger5 = BigInteger.ModPow(bigInteger5 * bigInteger8, 2, p);
                    value = BigInteger.ModPow(bigInteger8, 2, p);
                    ++bigInteger6;
                }

                return bigInteger4;
            }

            public static BigInteger ChineseRemainderTheorem(BigInteger[] n, BigInteger[] a)
            {
                BigInteger bigInteger = n.Aggregate(BigInteger.One, (BigInteger i, BigInteger j) => i * j);
                BigInteger bigInteger2 = 0;
                for (int k = 0; k < n.Length; k++)
                {
                    BigInteger bigInteger3 = bigInteger / n[k];
                    bigInteger2 += a[k] * ModularMultiplicativeInverse(bigInteger3, n[k]) * bigInteger3;
                }

                return bigInteger2 % bigInteger;
            }

            public static BigInteger ModularMultiplicativeInverse(BigInteger a, BigInteger m)
            {
                BigInteger bigInteger = a % m;
                for (int i = 1; i < m; i++)
                {
                    if (bigInteger * i % m == 1L)
                    {
                        return i;
                    }
                }

                return 1;
            }

            public static int EulersTotientPhi(int n)
            {
                if (n < 3)
                {
                    return 1;
                }

                if (n == 3)
                {
                    return 2;
                }

                int num = n;
                if ((n & 1) == 0)
                {
                    num >>= 1;
                    while (((n >>= 1) & 1) == 0)
                    {
                    }
                }

                for (int i = 3; i * i <= n; i += 2)
                {
                    if (n % i == 0)
                    {
                        num -= num / i;
                        while ((n /= i) % i == 0)
                        {
                        }
                    }
                }

                if (n > 1)
                {
                    num -= num / n;
                }

                return num;
            }

            public static double LaguerresMethod(Polynomial poly, double guess = 1.0, double maxIterations = 100.0, double precision = 1E-06)
            {
                if (poly.Degree < 1)
                {
                    throw new Exception("No root exists for a constant (degree 0) polynomial!");
                }

                double num = guess;
                double num2 = poly.Degree;
                Polynomial derivativePolynomial = GetDerivativePolynomial(poly);
                Polynomial derivativePolynomial2 = GetDerivativePolynomial(derivativePolynomial);
                int i;
                for (i = 0; (double)i < maxIterations; i++)
                {
                    if (!(Math.Abs(poly.Evaluate(num)) >= precision))
                    {
                        break;
                    }

                    double num3 = derivativePolynomial.Evaluate(num) / poly.Evaluate(num);
                    double num4 = Math.Pow(num3, 2.0);
                    double num5 = num4 - derivativePolynomial2.Evaluate(num) / poly.Evaluate(num);
                    double num6 = (num2 - 1.0) * (num2 * num5 - num4);
                    if (!(num6 >= 0.0))
                    {
                        break;
                    }

                    double num7 = Math.Sqrt(num6);
                    double num8 = num3 + num7;
                    double num9 = num3 - num7;
                    num7 = ((!(Math.Abs(num8) >= Math.Abs(num9))) ? num9 : num8);
                    double num10 = num2 / num7;
                    num -= num10;
                }

                if ((double)i == maxIterations)
                {
                    return double.NaN;
                }

                if (Math.Abs(poly.Evaluate(num)) >= precision)
                {
                    return double.NaN;
                }

                int digits = (int)Math.Abs(Math.Log10(precision));
                return Math.Round(num, digits);
            }

            public static Complex LaguerresMethod_Complex(Polynomial poly, double guess, double maxIterations, double precision)
            {
                if (poly.Degree < 1)
                {
                    throw new Exception("No root exists for a constant (degree 0) polynomial!");
                }

                Complex indeterminateValue = guess;
                double num = poly.Degree;
                Polynomial derivativePolynomial = GetDerivativePolynomial(poly);
                Polynomial derivativePolynomial2 = GetDerivativePolynomial(derivativePolynomial);
                int i;
                for (i = 0; (double)i < maxIterations; i++)
                {
                    if (Complex.Abs(poly.Evaluate(indeterminateValue)) < precision)
                    {
                        break;
                    }

                    Complex complex = derivativePolynomial.Evaluate(indeterminateValue) / poly.Evaluate(indeterminateValue);
                    Complex complex2 = Complex.Pow(complex, 2.0);
                    Complex complex3 = complex2 - derivativePolynomial2.Evaluate(indeterminateValue) / poly.Evaluate(indeterminateValue);
                    Complex value = (num - 1.0) * (num * complex3 - complex2);
                    if (!(Complex.Abs(value) >= 0.0))
                    {
                        break;
                    }

                    Complex complex4 = Complex.Sqrt(value);
                    Complex complex5 = complex + complex4;
                    Complex complex6 = complex - complex4;
                    complex4 = ((!(Complex.Abs(complex5) >= Complex.Abs(complex6))) ? complex6 : complex5);
                    Complex complex7 = num / complex4;
                    indeterminateValue -= complex7;
                }

                if ((double)i == maxIterations)
                {
                    return Complex.Zero;
                }

                int digits = (int)Math.Abs(Math.Log10(precision));
                double real = Math.Round(indeterminateValue.Real, digits);
                double imaginary = Math.Round(indeterminateValue.Imaginary, digits);
                return new Complex(real, imaginary);
            }
        }

        public static class Field
        {
            public static Polynomial GCD(Polynomial left, Polynomial right, BigInteger modulus)
            {
                Polynomial polynomial = left.Clone();
                Polynomial polynomial2 = right.Clone();
                if (polynomial2.Degree > polynomial.Degree)
                {
                    Polynomial polynomial3 = polynomial2;
                    polynomial2 = polynomial;
                    polynomial = polynomial3;
                }

                while (polynomial2.Terms.Length != 0 && !(polynomial2.Terms[0].CoEfficient == 0L))
                {
                    Polynomial toReduce = polynomial;
                    polynomial = polynomial2;
                    polynomial2 = ModMod(toReduce, polynomial2, modulus);
                }

                if (polynomial.Degree == 0)
                {
                    return One.Clone();
                }

                return polynomial;
            }

            public static Polynomial ModMod(Polynomial toReduce, Polynomial modPoly, BigInteger primeModulus)
            {
                return Modulus(Modulus(toReduce, modPoly), primeModulus);
            }

            public static Polynomial Modulus(Polynomial poly, Polynomial mod)
            {
                int num = mod.CompareTo(poly);
                if (num > 0)
                {
                    return poly.Clone();
                }

                if (num == 0)
                {
                    return Zero.Clone();
                }

                Polynomial.Divide(poly, mod, out Polynomial remainder);
                return remainder;
            }

            public static Polynomial Modulus(Polynomial poly, BigInteger mod)
            {
                Polynomial polynomial = poly.Clone();
                List<Term> list = new List<Term>();
                Term[] terms = polynomial.Terms;
                foreach (Term term in terms)
                {
                    BigInteger.DivRem(term.CoEfficient, mod, out var remainder);
                    if (remainder.Sign == -1)
                    {
                        remainder += mod;
                    }

                    list.Add(new Term(remainder, term.Exponent));
                }

                Term[] array = list.SkipWhile((Term t) => t.CoEfficient.Sign == 0).ToArray();
                if (!array.Any())
                {
                    array = Term.GetTerms(new BigInteger[1] { 0 });
                }

                return new Polynomial(array);
            }

            public static Polynomial Divide(Polynomial left, Polynomial right, BigInteger mod, out Polynomial remainder)
            {
                if (left == null)
                {
                    throw new ArgumentNullException("left");
                }

                if (right == null)
                {
                    throw new ArgumentNullException("right");
                }

                if (right.Degree > left.Degree || right.CompareTo(left) == 1)
                {
                    remainder = Zero;
                    return left.Clone();
                }

                int degree = right.Degree;
                int num = left.Degree - degree + 1;
                BigInteger divisor = right[degree].Clone().Mod(mod);
                Polynomial polynomial = left.Clone();
                Polynomial polynomial2 = Zero.Clone();
                for (int num2 = num - 1; num2 >= 0; num2--)
                {
                    polynomial2[num2] = BigInteger.Divide(polynomial[degree + num2], divisor).Mod(mod);
                    polynomial[degree + num2] = new BigInteger(0);
                    for (int num3 = degree + num2 - 1; num3 >= num2; num3--)
                    {
                        polynomial[num3] = BigInteger.Subtract(polynomial[num3], BigInteger.Multiply(polynomial2[num2], right[num3 - num2]).Mod(mod)).Mod(mod);
                    }
                }

                polynomial.RemoveZeros();
                polynomial2.RemoveZeros();
                remainder = polynomial.Clone();
                return polynomial2.Clone();
            }

            public static Polynomial Multiply(Polynomial poly, BigInteger multiplier, BigInteger mod)
            {
                Polynomial polynomial = poly.Clone();
                Term[] terms = polynomial.Terms;
                foreach (Term term in terms)
                {
                    BigInteger coEfficient = term.CoEfficient;
                    if (coEfficient != 0L)
                    {
                        coEfficient *= multiplier;
                        term.CoEfficient = coEfficient.Mod(mod);
                    }
                }

                return polynomial;
            }

            public static Polynomial PowMod(Polynomial poly, BigInteger exponent, BigInteger mod)
            {
                Polynomial polynomial = poly.Clone();
                Term[] terms = polynomial.Terms;
                foreach (Term term in terms)
                {
                    BigInteger coEfficient = term.CoEfficient;
                    if (coEfficient != 0L)
                    {
                        coEfficient = BigInteger.ModPow(coEfficient, exponent, mod);
                        if (coEfficient.Sign == -1)
                        {
                            throw new Exception("BigInteger.ModPow returned negative number");
                        }

                        term.CoEfficient = coEfficient;
                    }
                }

                return polynomial;
            }

            public static Polynomial ExponentiateMod(Polynomial startPoly, BigInteger s2, Polynomial f, BigInteger p)
            {
                Polynomial polynomial = One.Clone();
                if (s2 == 0L)
                {
                    return polynomial;
                }

                Polynomial polynomial2 = startPoly.Clone();
                bool[] array = new BitArray(s2.ToByteArray()).Cast<bool>().ToArray();
                if (array[0])
                {
                    polynomial = startPoly.Clone();
                }

                int i = 1;
                for (int num = array.Length; i < num; i++)
                {
                    polynomial2 = ModMod(Square(polynomial2), f, p);
                    if (array[i])
                    {
                        polynomial = ModMod(Polynomial.Multiply(polynomial2, polynomial), f, p);
                    }
                }

                return polynomial;
            }

            public static Polynomial ModPow(Polynomial poly, BigInteger exponent, Polynomial mod)
            {
                if (exponent < 0L)
                {
                    throw new NotImplementedException("Raising a polynomial to a negative exponent not supported. Build this functionality if it is needed.");
                }

                if (exponent == 0L)
                {
                    return One;
                }

                if (exponent == 1L)
                {
                    return poly.Clone();
                }

                if (exponent == 2L)
                {
                    return Square(poly);
                }

                Polynomial polynomial = Square(poly);
                for (BigInteger bigInteger = exponent - 2; bigInteger != 0L; bigInteger -= (BigInteger)1)
                {
                    polynomial = Polynomial.Multiply(poly, polynomial);
                    if (polynomial.CompareTo(mod) < 0)
                    {
                        polynomial = Modulus(polynomial, mod);
                    }
                }

                return polynomial;
            }

            public static bool IsIrreducibleOverField(Polynomial f, BigInteger p)
            {
                return Polynomial.GCD(ModMod(new Polynomial(new Term[2]
                {
                    new Term(1, (int)p),
                    new Term(-1, 1)
                }), f, p), f).CompareTo(One) == 0;
            }

            public static bool IsIrreducibleOverP(Polynomial poly, BigInteger p)
            {
                List<BigInteger> list = poly.Terms.Select((Term t) => t.CoEfficient).ToList();
                BigInteger bigInteger = list.Last();
                BigInteger bigInteger2 = list.First();
                list.Remove(bigInteger);
                list.Remove(bigInteger2);
                BigInteger.DivRem(bigInteger, p, out var remainder);
                BigInteger.DivRem(bigInteger2, p.Square(), out var remainder2);
                bool num = (remainder != 0L) & (remainder2 != 0L);
                list.Add(p);
                return num & (list.GCD() == 1L);
            }
        }

        public static Polynomial Zero;

        public static Polynomial One;

        public static Polynomial Two;

        [DataMember(Name = "Terms")]
        [DebuggerBrowsable(DebuggerBrowsableState.Never)]
        private List<Term> _terms = new List<Term>();

        [DebuggerBrowsable(DebuggerBrowsableState.RootHidden)]
        public Term[] Terms => _terms.ToArray();

        public int Degree
        {
            get
            {
                if (_terms.Any())
                {
                    return _terms.Max((Term term) => term.Exponent);
                }

                return 0;
            }
        }

        public BigInteger this[int degree]
        {
            get
            {
                return Terms.FirstOrDefault((Term t) => t.Exponent == degree)?.CoEfficient ?? BigInteger.Zero;
            }
            set
            {
                Term term = Terms.FirstOrDefault((Term t) => t.Exponent == degree);
                if (term == null)
                {
                    if (value != BigInteger.Zero)
                    {
                        Term item = new Term(value, degree);
                        List<Term> terms = _terms;
                        terms.Add(item);
                        SetTerms(terms);
                    }
                }
                else
                {
                    term.CoEfficient = value;
                }
            }
        }

        static Polynomial()
        {
            Zero = new Polynomial(Term.GetTerms(new BigInteger[1]
            {
                new BigInteger(0)
            }));
            One = new Polynomial(Term.GetTerms(new BigInteger[1]
            {
                new BigInteger(1)
            }));
            Two = new Polynomial(Term.GetTerms(new BigInteger[1]
            {
                new BigInteger(2)
            }));
        }

        public Polynomial()
        {
            _terms = new List<Term>();
        }

        public Polynomial(Term[] terms)
        {
            SetTerms(terms);
        }

        public Polynomial(BigInteger n, BigInteger polynomialBase)
            : this(n, polynomialBase, (int)Math.Truncate(BigInteger.Log(n, (double)polynomialBase) + 1.0))
        {
        }

        public Polynomial(BigInteger n, BigInteger polynomialBase, int forceDegree)
        {
            SetTerms(GetPolynomialTerms(n, polynomialBase, forceDegree));
        }

        private void SetTerms(IEnumerable<Term> terms)
        {
            _terms = terms.OrderBy((Term t) => t.Exponent).ToList();
            RemoveZeros();
        }

        private void RemoveZeros()
        {
            _terms.RemoveAll((Term t) => t.CoEfficient == 0L);
            if (!_terms.Any())
            {
                _terms = Term.GetTerms(new BigInteger[1] { 0 }).ToList();
            }
        }

        private static List<Term> GetPolynomialTerms(BigInteger value, BigInteger polynomialBase, int degree)
        {
            int num = degree;
            BigInteger bigInteger = value;
            List<Term> list = new List<Term>();
            while (num >= 0 && bigInteger > 0L)
            {
                BigInteger bigInteger2 = BigInteger.Pow(polynomialBase, num);
                if (bigInteger2 == 1L)
                {
                    list.Add(new Term(bigInteger, num));
                    bigInteger = 0;
                }
                else if (bigInteger2 == bigInteger)
                {
                    list.Add(new Term(1, num));
                    bigInteger -= bigInteger2;
                }
                else if (bigInteger2 < BigInteger.Abs(bigInteger))
                {
                    BigInteger bigInteger3 = BigInteger.Divide(bigInteger, bigInteger2);
                    if (bigInteger3 > bigInteger2)
                    {
                        bigInteger3 = bigInteger2;
                    }

                    list.Add(new Term(bigInteger3, num));
                    BigInteger bigInteger4 = BigInteger.Multiply(bigInteger3, bigInteger2);
                    bigInteger -= bigInteger4;
                }

                num--;
            }

            return list.ToList();
        }

        public static Polynomial FromRoots(params BigInteger[] roots)
        {
            return Product(roots.Select((BigInteger root) => new Polynomial(new Term[2]
            {
                new Term(1, 1),
                new Term(BigInteger.Negate(root), 0)
            })));
        }

        public static Polynomial Parse(string input)
        {
            if (string.IsNullOrWhiteSpace(input))
            {
                throw new ArgumentException();
            }

            string[] array = input.Replace(" ", "").Replace("−", "-").Replace("-", "+-")
                .Split(new char[1] { '+' }, StringSplitOptions.RemoveEmptyEntries);
            if (!array.Any())
            {
                throw new FormatException();
            }

            List<Term> list = new List<Term>();
            string[] array2 = array;
            for (int i = 0; i < array2.Length; i++)
            {
                Term item = Term.Parse(array2[i]);
                list.Add(item);
            }

            if (!list.Any())
            {
                throw new FormatException();
            }

            return new Polynomial(list.ToArray());
        }

        public BigInteger Evaluate(BigInteger indeterminateValue)
        {
            return Evaluate(this, indeterminateValue);
        }

        public double Evaluate(double indeterminateValue)
        {
            return Evaluate(this, indeterminateValue);
        }

        public decimal Evaluate(decimal indeterminateValue)
        {
            return Evaluate(this, indeterminateValue);
        }

        public Complex Evaluate(Complex indeterminateValue)
        {
            return Evaluate(this, indeterminateValue);
        }

        public static BigInteger Evaluate(Polynomial polynomial, BigInteger indeterminateValue)
        {
            int num = polynomial.Degree;
            BigInteger result = polynomial[num];
            while (--num >= 0)
            {
                result *= indeterminateValue;
                result += polynomial[num];
            }

            return result;
        }

        public static double Evaluate(Polynomial polynomial, double indeterminateValue)
        {
            int num = polynomial.Degree;
            double num2 = (double)polynomial[num];
            while (--num >= 0)
            {
                num2 *= indeterminateValue;
                num2 += (double)polynomial[num];
            }

            return num2;
        }

        public static decimal Evaluate(Polynomial polynomial, decimal indeterminateValue)
        {
            int num = polynomial.Degree;
            decimal result = (decimal)polynomial[num];
            while (--num >= 0)
            {
                result *= indeterminateValue;
                result += (decimal)polynomial[num];
            }

            return result;
        }

        public static Complex Evaluate(Polynomial polynomial, Complex indeterminateValue)
        {
            int num = polynomial.Degree;
            Complex result = (Complex)polynomial[num];
            while (--num >= 0)
            {
                result *= indeterminateValue;
                result += (Complex)polynomial[num];
            }

            return result;
        }

        public Polynomial FunctionalComposition(Polynomial indeterminateValue)
        {
            List<Term> list = Terms.ToList();
            List<Polynomial> list2 = new List<Polynomial>();
            foreach (Term item2 in list)
            {
                Polynomial item = Multiply(new Polynomial(new Term[1]
                {
                    new Term(item2.CoEfficient, 0)
                }), Pow(indeterminateValue, item2.Exponent));
                list2.Add(item);
            }

            return Sum(list2);
        }

        public static List<Polynomial> Factor(Polynomial polynomial)
        {
            List<Polynomial> list = new List<Polynomial>();
            Polynomial polynomial2 = polynomial.Clone();
            BigInteger bigInteger = polynomial2.Terms.Select((Term trm) => trm.CoEfficient).Aggregate(new Func<BigInteger, BigInteger, BigInteger>(BigInteger.GreatestCommonDivisor));
            if (bigInteger > 1L)
            {
                Polynomial polynomial3 = Parse(bigInteger.ToString());
                list.Add(polynomial3);
                polynomial2 = Divide(polynomial2, polynomial3);
            }

            BigInteger coEfficient = polynomial2.Terms.Last().CoEfficient;
            BigInteger coEfficient2 = polynomial2.Terms.First().CoEfficient;
            if (coEfficient == 0L)
            {
                throw new Exception("Leading coefficient is zero!?");
            }

            List<BigInteger> allDivisors = GetAllDivisors(coEfficient2);
            List<BigInteger> allDivisors2 = GetAllDivisors(coEfficient);
            allDivisors.AddRange(from n in allDivisors.ToList()
                                 select BigInteger.Negate(n));
            if (allDivisors2.Count > 1)
            {
                allDivisors2.AddRange(from n in allDivisors2.ToList()
                                      select BigInteger.Negate(n));
            }

            foreach (BigInteger item in allDivisors2)
            {
                foreach (BigInteger item2 in allDivisors)
                {
                    double indeterminateValue = (double)item2 / (double)item;
                    if (polynomial2.Evaluate(indeterminateValue) == 0.0)
                    {
                        int value = (int)BigInteger.Negate(item2);
                        string arg = ((Math.Sign(value) == -1) ? "-" : "+");
                        string arg2 = ((item == 1L) ? "" : $"{item}*");
                        Polynomial polynomial4 = Parse($"{arg2}X {arg} {Math.Abs(value)}");
                        list.Add(polynomial4);
                        polynomial2 = Divide(polynomial2, polynomial4);
                        if (polynomial2 == One)
                        {
                            return list;
                        }
                    }
                }
            }

            if (polynomial2 != One)
            {
                list.Add(polynomial2);
            }

            return list;
        }

        private static List<BigInteger> GetAllDivisors(BigInteger value)
        {
            BigInteger bigInteger = value;
            if (BigInteger.Abs(bigInteger) == 1L)
            {
                return new List<BigInteger> { bigInteger };
            }

            List<BigInteger> list = new List<BigInteger>();
            if (bigInteger.Sign == -1)
            {
                list.Add(-1);
                bigInteger *= BigInteger.MinusOne;
            }

            for (BigInteger bigInteger2 = 1; bigInteger2 * bigInteger2 < bigInteger; ++bigInteger2)
            {
                if (bigInteger % bigInteger2 == 0L)
                {
                    list.Add(bigInteger2);
                }
            }

            for (BigInteger bigInteger3 = bigInteger.SquareRoot(); bigInteger3 >= 1L; --bigInteger3)
            {
                if (bigInteger % bigInteger3 == 0L)
                {
                    list.Add(bigInteger / bigInteger3);
                }
            }

            return list;
        }

        public static Polynomial GetDerivativePolynomial(Polynomial polynomial)
        {
            List<Term> list = new List<Term>();
            Term[] terms = polynomial.Terms;
            foreach (Term term in terms)
            {
                int num = term.Exponent - 1;
                if (num >= 0)
                {
                    list.Add(new Term(term.CoEfficient * term.Exponent, num));
                }
            }

            return new Polynomial(list.ToArray());
        }

        public Polynomial IndefiniteIntegral(BigInteger c)
        {
            List<Term> list = new List<Term>();
            list.Add(new Term(c, 0));
            BigInteger[] array = new BigInteger[Degree + 2];
            array[0] = c;
            for (int i = 0; i <= Degree; i++)
            {
                array[i + 1] = this[i] / (i + 1);
                list.Add(new Term(this[i] / (i + 1), i + 1));
            }

            return new Polynomial(list.ToArray());
        }

        public static Polynomial GetReciprocalPolynomial(Polynomial polynomial)
        {
            List<Term> list = new List<Term>();
            int exponentIndex;
            for (exponentIndex = 0; exponentIndex <= polynomial.Degree; exponentIndex++)
            {
                Term term = polynomial.Terms.Where((Term trm) => trm.Exponent == exponentIndex).FirstOrDefault();
                if (term == null)
                {
                    term = new Term(0, exponentIndex);
                }

                list.Add(term);
            }

            List<Term> list2 = new List<Term>();
            exponentIndex = 0;
            for (int num = polynomial.Degree; num >= 0; num--)
            {
                BigInteger coefficient = polynomial[num];
                int exponent = list[exponentIndex].Exponent;
                Term item = new Term(coefficient, exponent);
                list2.Add(item);
                exponentIndex++;
            }

            return new Polynomial(list2.ToArray());
        }

        public static Polynomial MakeMonic(Polynomial polynomial, BigInteger polynomialBase)
        {
            int degree = polynomial.Degree;
            Polynomial polynomial2 = polynomial.Clone();
            if (BigInteger.Abs(polynomial2.Terms[degree].CoEfficient) > 1L)
            {
                BigInteger bigInteger = (polynomial2.Terms[degree].CoEfficient - 1) * polynomialBase;
                polynomial2.Terms[degree].CoEfficient = 1;
                polynomial2.Terms[degree - 1].CoEfficient += bigInteger;
            }

            return polynomial2;
        }

        public static Polynomial MakeCoefficientsSmaller(Polynomial polynomial, BigInteger polynomialBase)
        {
            BigInteger bigInteger = polynomialBase / 2;
            Polynomial polynomial2 = polynomial.Clone();
            int i = 0;
            for (int degree = polynomial2.Degree; i <= degree; i++)
            {
                if (polynomial2[i] > bigInteger)
                {
                    polynomial2[i + 1] += (BigInteger)1;
                    polynomial2[i] = -(polynomialBase - polynomial2[i]);
                }
            }

            return polynomial2.Clone();
        }

        public static Polynomial GCD(Polynomial left, Polynomial right)
        {
            List<Polynomial> list = Factor(left);
            List<Polynomial> list2 = Factor(right);
            Polynomial polynomial = One;
            foreach (Polynomial item in list)
            {
                if (list2.Contains(item))
                {
                    polynomial = Multiply(polynomial, item);
                }
            }

            return polynomial;
        }

        public static Polynomial Divide(Polynomial left, Polynomial right)
        {
            Polynomial remainder;
            return Divide(left, right, out remainder);
        }

        public static Polynomial Divide(Polynomial left, Polynomial right, out Polynomial remainder)
        {
            if (left == null)
            {
                throw new ArgumentNullException("left");
            }

            if (right == null)
            {
                throw new ArgumentNullException("right");
            }

            if (right.Degree > left.Degree || right.CompareTo(left) == 1)
            {
                remainder = new Polynomial(new Term[1]
                {
                    new Term(new BigInteger(0), 0)
                });
                return left.Clone();
            }

            int degree = right.Degree;
            int num = left.Degree - degree + 1;
            BigInteger divisor = right[degree].Clone();
            Polynomial polynomial = left.Clone();
            Polynomial polynomial2 = new Polynomial(new Term[1]
            {
                new Term(new BigInteger(0), 0)
            });
            for (int num2 = num - 1; num2 >= 0; num2--)
            {
                polynomial2[num2] = BigInteger.Divide(polynomial[degree + num2], divisor);
                polynomial[degree + num2] = new BigInteger(0);
                for (int num3 = degree + num2 - 1; num3 >= num2; num3--)
                {
                    polynomial[num3] = BigInteger.Subtract(polynomial[num3], BigInteger.Multiply(polynomial2[num2], right[num3 - num2]));
                }
            }

            polynomial.RemoveZeros();
            polynomial2.RemoveZeros();
            remainder = polynomial.Clone();
            return polynomial2.Clone();
        }

        public static Polynomial Multiply(Polynomial left, Polynomial right)
        {
            if (left == null)
            {
                throw new ArgumentNullException("left");
            }

            if (right == null)
            {
                throw new ArgumentNullException("right");
            }

            BigInteger[] array = new BigInteger[left.Degree + right.Degree + 1];
            for (int i = 0; i <= left.Degree; i++)
            {
                for (int j = 0; j <= right.Degree; j++)
                {
                    array[i + j] += BigInteger.Multiply(left[i], right[j]);
                }
            }

            return new Polynomial(Term.GetTerms(array));
        }

        public static Polynomial Product(params Polynomial[] polys)
        {
            return Product(polys.ToList());
        }

        public static Polynomial Product(IEnumerable<Polynomial> polys)
        {
            Polynomial polynomial = null;
            foreach (Polynomial poly in polys)
            {
                polynomial = ((polynomial != null) ? Multiply(polynomial, poly) : poly.Clone());
            }

            return polynomial;
        }

        public static Polynomial Square(Polynomial polynomial)
        {
            return Multiply(polynomial, polynomial);
        }

        public static Polynomial Pow(Polynomial polynomial, int exponent)
        {
            if (exponent < 0)
            {
                throw new NotImplementedException("Raising a polynomial to a negative exponent not supported. Build this functionality if it is needed.");
            }

            switch (exponent)
            {
                case 0:
                    return new Polynomial(new Term[1]
                    {
                    new Term(1, 0)
                    });
                case 1:
                    return polynomial.Clone();
                case 2:
                    return Square(polynomial);
                default:
                    {
                        Polynomial polynomial2 = Square(polynomial);
                        for (int num = exponent - 2; num != 0; num--)
                        {
                            polynomial2 = Multiply(polynomial2, polynomial);
                        }

                        return polynomial2;
                    }
            }
        }

        public static Polynomial Subtract(Polynomial left, Polynomial right)
        {
            if (left == null)
            {
                throw new ArgumentNullException("left");
            }

            if (right == null)
            {
                throw new ArgumentNullException("right");
            }

            BigInteger[] array = new BigInteger[Math.Max(left.Degree, right.Degree) + 1];
            for (int i = 0; i < array.Length; i++)
            {
                BigInteger bigInteger = left[i];
                BigInteger bigInteger2 = right[i];
                array[i] = bigInteger - bigInteger2;
            }

            return new Polynomial(Term.GetTerms(array.ToArray()));
        }

        public static Polynomial Sum(params Polynomial[] polys)
        {
            return Sum(polys.ToList());
        }

        public static Polynomial Sum(IEnumerable<Polynomial> polys)
        {
            Polynomial polynomial = null;
            foreach (Polynomial poly in polys)
            {
                polynomial = ((polynomial != null) ? Add(polynomial, poly) : poly.Clone());
            }

            return polynomial;
        }

        public static Polynomial Add(Polynomial left, Polynomial right)
        {
            if (left == null)
            {
                throw new ArgumentNullException("left");
            }

            if (right == null)
            {
                throw new ArgumentNullException("right");
            }

            BigInteger[] array = new BigInteger[Math.Max(left.Degree, right.Degree) + 1];
            for (int i = 0; i < array.Length; i++)
            {
                array[i] = left[i] + right[i];
            }

            return new Polynomial(Term.GetTerms(array.ToArray()));
        }

        public int CompareTo(object obj)
        {
            if (obj == null)
            {
                throw new NullReferenceException();
            }

            Polynomial polynomial = obj as Polynomial;
            if (polynomial == null)
            {
                throw new ArgumentException();
            }

            return CompareTo(polynomial);
        }

        public int CompareTo(Polynomial other)
        {
            if (other == null)
            {
                throw new ArgumentException();
            }

            if (other.Degree != Degree)
            {
                if (other.Degree > Degree)
                {
                    return -1;
                }

                return 1;
            }

            for (int num = Degree; num >= 0; num--)
            {
                BigInteger bigInteger = this[num];
                BigInteger bigInteger2 = other[num];
                if (bigInteger < bigInteger2)
                {
                    return -1;
                }

                if (bigInteger > bigInteger2)
                {
                    return 1;
                }
            }

            return 0;
        }

        public Polynomial Clone()
        {
            return new Polynomial(_terms.Select((Term pt) => pt.Clone()).ToArray());
        }

        public bool Equals(Polynomial other)
        {
            return CompareTo(other) == 0;
        }

        private static int CombineHashCodes(int h1, int h2)
        {
            return ((h1 << 5) + h1) ^ h2;
        }

        public override int GetHashCode()
        {
            int num = Degree.GetHashCode();
            Term[] terms = Terms;
            foreach (Term term in terms)
            {
                num = CombineHashCodes(num, term.GetHashCode());
            }

            return num;
        }

        public override bool Equals(object obj)
        {
            if (obj == null)
            {
                return false;
            }

            Polynomial polynomial = obj as Polynomial;
            if (polynomial == null)
            {
                return false;
            }

            return Equals(polynomial);
        }

        private static string FormatString(Polynomial polynomial)
        {
            List<string> list = new List<string>();
            int num = polynomial.Terms.Length;
            while (--num >= 0)
            {
                Term term = polynomial.Terms[num];
                if (term.CoEfficient == 0L)
                {
                    if (term.Exponent == 0 && list.Count == 0)
                    {
                        list.Add("0");
                    }

                    continue;
                }

                switch (term.Exponent)
                {
                    case 0:
                        list.Add($"{term.CoEfficient}");
                        break;
                    case 1:
                        if (term.CoEfficient == 1L)
                        {
                            list.Add("X");
                        }
                        else if (term.CoEfficient == -1L)
                        {
                            list.Add("-X");
                        }
                        else
                        {
                            list.Add($"{term.CoEfficient}*X");
                        }

                        break;
                    default:
                        if (term.CoEfficient == 1L)
                        {
                            list.Add($"X^{term.Exponent}");
                        }
                        else if (term.CoEfficient == -1L)
                        {
                            list.Add($"-X^{term.Exponent}");
                        }
                        else
                        {
                            list.Add($"{term.CoEfficient}*X^{term.Exponent}");
                        }

                        break;
                }
            }

            return string.Join(" + ", list).Replace("+ -", "- ");
        }

        public override string ToString()
        {
            return FormatString(this);
        }
    }
}