using System;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;
using ExtendedArithmetic;

namespace GNFSCore.SquareRoot
{
	using IntegerMath;

	public static class FiniteFieldArithmetic
	{
		/// <summary>
		/// Tonelli-Shanks algorithm for finding polynomial modular square roots
		/// </summary>
		/// <returns></returns>
		public static Polynomial SquareRoot(Polynomial startPolynomial, Polynomial f, BigInteger p, int degree, BigInteger m)
		{
			BigInteger q = BigInteger.Pow(p, degree);
			BigInteger s = q - 1;

			int r = 0;
			while (s.Mod(2) == 0)
			{
				s /= 2;
				r++;
			}

			BigInteger halfS = ((s + 1) / 2);
			if (r == 1 && q.Mod(4) == 3)
			{
				halfS = ((q + 1) / 4);
			}

			BigInteger quadraticNonResidue = Legendre.SymbolSearch(m + 1, q, -1);
			BigInteger theta = quadraticNonResidue;
			BigInteger minusOne = BigInteger.ModPow(theta, ((q - 1) / 2), p);

			Polynomial omegaPoly = Polynomial.Field.ExponentiateMod(startPolynomial, halfS, f, p);

			BigInteger lambda = minusOne;
			BigInteger zeta = 0;

			int i = 0;
			do
			{
				i++;

				zeta = BigInteger.ModPow(theta, (i * s), p);

				lambda = (lambda * BigInteger.Pow(zeta, (int)Math.Pow(2, (r - i)))).Mod(p);

				omegaPoly = Polynomial.Field.Multiply(omegaPoly, BigInteger.Pow(zeta, (int)Math.Pow(2, ((r - i) - 1))), p);
			}
			while (!((lambda == 1) || (i > (r))));

			return omegaPoly;
		}

		/// <summary>
		/// Finds X such that a*X = 1 (mod p)
		/// </summary>
		/// <param name="a">a.</param>
		/// <param name="p">The modulus</param>
		/// <returns></returns>
		public static BigInteger ModularMultiplicativeInverse(BigInteger a, BigInteger p)
		{
			if (p == 1)
			{
				return 0;
			}

			BigInteger divisor;
			BigInteger dividend = a;
			BigInteger diff = 0;
			BigInteger result = 1;
			BigInteger quotient = 0;
			BigInteger lastDivisor = 0;
			BigInteger remainder = p;

			while (dividend > 1)
			{
				divisor = remainder;
				quotient = BigInteger.DivRem(dividend, divisor, out remainder); // Divide             
				dividend = divisor;
				lastDivisor = diff; // The thing to divide will be the last divisor

				// Update diff and result 
				diff = result - quotient * diff;
				result = lastDivisor;
			}

			if (result < 0)
			{
				result += p; // Make result positive 
			}
			return result;
		}

		/// <summary>
		/// Finds N such that primes[i] ≡ values[i] (mod N) for all values[i] with 0 &lt; i &lt; a.Length
		/// </summary>
		public static BigInteger ChineseRemainder(List<BigInteger> primes, List<BigInteger> values)
		{
			BigInteger primeProduct = primes.Product();

			int indx = 0;
			BigInteger Z = 0;
			foreach (BigInteger pi in primes)
			{
				BigInteger Pj = primeProduct / pi;
				BigInteger Aj = ModularMultiplicativeInverse(Pj, pi);
				BigInteger AXPj = values[indx] * Aj * Pj;

				Z += AXPj;
				indx++;
			}

			BigInteger r = Z / primeProduct;
			BigInteger rP = r * primeProduct;
			BigInteger finalResult_sqrt = (Z - rP);
			return finalResult_sqrt;
		}

		/// <summary>
		/// Reduce a polynomial by a modulus polynomial and modulus integer.
		/// </summary>
		public static Polynomial ModMod(Polynomial toReduce, Polynomial modPoly, BigInteger primeModulus)
		{
			int compare = modPoly.CompareTo(toReduce);
			if (compare > 0)
			{
				return toReduce;
			}
			if (compare == 0)
			{
				return Polynomial.Zero;
			}

			return Remainder(toReduce, modPoly, primeModulus);
		}

		public static Polynomial Remainder(Polynomial left, Polynomial right, BigInteger mod)
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
				return Polynomial.Zero.Clone();
			}

			int rightDegree = right.Degree;
			int quotientDegree = left.Degree - rightDegree + 1;

			BigInteger leadingCoefficent = right[rightDegree].Mod(mod);
			if (leadingCoefficent != 1) { throw new ArgumentNullException("right", "This method was expecting only monomials (leading coefficient is 1) for the right-hand-side polynomial."); }

			Polynomial rem = left.Clone();
			BigInteger quot = 0;

			for (int i = quotientDegree - 1; i >= 0; i--)
			{
				quot = BigInteger.Remainder(rem[rightDegree + i], mod);//.Mod(mod);

				rem[rightDegree + i] = 0;

				for (int j = rightDegree + i - 1; j >= i; j--)
				{
					rem[j] = BigInteger.Subtract(
													rem[j],
													BigInteger.Multiply(quot, right[j - i]).Mod(mod)
												).Mod(mod);
				}
			}

			return new Polynomial(rem.Terms);
		}
	}
}

using System;
using System.Linq;
using System.Text;
using System.Numerics;
using System.Threading;
using System.Collections.Generic;
using ExtendedArithmetic;

namespace GNFSCore.SquareRoot
{
	using ExtendedNumerics.Internal;
	using IntegerMath;
	using System.IO;
	using System.Net.Http.Headers;
	using System.Text.RegularExpressions;
	using static GNFSCore.GNFS;

	public partial class SquareFinder
	{
		public BigInteger RationalProduct { get; set; }
		public BigInteger RationalSquare { get; set; }
		public BigInteger RationalSquareRootResidue { get; set; }
		public bool IsRationalSquare { get; set; }
		public bool IsRationalIrreducible { get; set; }

		public BigInteger AlgebraicProduct { get; set; }
		public BigInteger AlgebraicSquare { get; set; }
		public BigInteger AlgebraicProductModF { get; set; }
		public BigInteger AlgebraicSquareResidue { get; set; }
		public BigInteger AlgebraicSquareRootResidue { get; set; }
		public List<BigInteger> AlgebraicPrimes { get; set; }
		public List<BigInteger> AlgebraicResults { get; set; }
		public bool IsAlgebraicSquare { get; set; }
		public bool IsAlgebraicIrreducible { get; set; }

		public BigInteger N { get; set; }
		public Polynomial S { get; set; }
		public Polynomial TotalS { get; set; }
		public List<Tuple<BigInteger, BigInteger>> RootsOfS { get; set; }
		public Polynomial PolynomialRing { get; set; }
		public List<Polynomial> PolynomialRingElements { get; set; }

		public BigInteger PolynomialBase { get; set; }
		public Polynomial MonicPolynomial { get; set; }
		public Polynomial PolynomialDerivative { get; set; }
		public Polynomial MonicPolynomialDerivative { get; set; }

		public Polynomial PolynomialDerivativeSquared { get; set; }
		public Polynomial PolynomialDerivativeSquaredInField { get; set; }

		public BigInteger PolynomialDerivativeValue { get; set; }
		public BigInteger PolynomialDerivativeValueSquared { get; set; }


		public Polynomial MonicPolynomialDerivativeSquared { get; set; }
		public Polynomial MonicPolynomialDerivativeSquaredInField { get; set; }

		public BigInteger MonicPolynomialDerivativeValue { get; set; }
		public BigInteger MonicPolynomialDerivativeValueSquared { get; set; }

		private GNFS gnfs { get; set; }
		private List<BigInteger> rationalNorms { get; set; }
		private List<BigInteger> algebraicNormCollection { get; set; }
		private List<Relation> relationsSet { get; set; }

		private LogMessageDelegate LogFunction;

		public SquareFinder(GNFS sieve)
		{
			LogFunction = sieve.LogMessage;

			RationalSquareRootResidue = -1;
			RootsOfS = new List<Tuple<BigInteger, BigInteger>>();

			gnfs = sieve;
			N = gnfs.N;
			PolynomialBase = gnfs.PolynomialBase;

			PolynomialDerivative = Polynomial.GetDerivativePolynomial(gnfs.CurrentPolynomial);
			PolynomialDerivativeSquared = Polynomial.Square(PolynomialDerivative);
			PolynomialDerivativeSquaredInField = Polynomial.Field.Modulus(PolynomialDerivativeSquared, gnfs.CurrentPolynomial);

			LogFunction.Invoke("");
			LogFunction.Invoke($"ƒ'(θ) = {PolynomialDerivative}");
			LogFunction.Invoke($"ƒ'(θ)² = {PolynomialDerivativeSquared}");
			LogFunction.Invoke($"ƒ'(θ)² ∈ ℤ[θ] = {PolynomialDerivativeSquaredInField}");

			PolynomialDerivativeValue = PolynomialDerivative.Evaluate(gnfs.PolynomialBase);
			PolynomialDerivativeValueSquared = BigInteger.Pow(PolynomialDerivativeValue, 2);

			LogFunction.Invoke("");
			LogFunction.Invoke($"ƒ'(m) = {PolynomialDerivativeValue}");
			LogFunction.Invoke($"ƒ'(m)² = {PolynomialDerivativeValueSquared}");


			MonicPolynomial = Polynomial.MakeMonic(gnfs.CurrentPolynomial, PolynomialBase);
			MonicPolynomialDerivative = Polynomial.GetDerivativePolynomial(MonicPolynomial);
			MonicPolynomialDerivativeSquared = Polynomial.Square(MonicPolynomialDerivative);
			MonicPolynomialDerivativeSquaredInField = Polynomial.Field.Modulus(MonicPolynomialDerivativeSquared, MonicPolynomial);

			MonicPolynomialDerivativeValue = MonicPolynomialDerivative.Evaluate(gnfs.PolynomialBase);
			MonicPolynomialDerivativeValueSquared = MonicPolynomialDerivativeSquared.Evaluate(gnfs.PolynomialBase);

			LogFunction.Invoke("");
			LogFunction.Invoke($"MonicPolynomial: {MonicPolynomial}");
			LogFunction.Invoke($"MonicPolynomialDerivative: {MonicPolynomialDerivative}");
			LogFunction.Invoke($"MonicPolynomialDerivativeSquared: {MonicPolynomialDerivativeSquared}");
			LogFunction.Invoke($"MonicPolynomialDerivativeSquaredInField: {MonicPolynomialDerivativeSquaredInField}");
		}

		private static bool IsPrimitive(IEnumerable<BigInteger> coefficients)
		{
			return (GCD.FindGCD(coefficients) == 1);
		}

		public static bool Solve(CancellationToken cancelToken, GNFS gnfs)
		{
			List<int> triedFreeRelationIndices = new List<int>();

			BigInteger polyBase = gnfs.PolynomialBase;
			List<List<Relation>> freeRelations = gnfs.CurrentRelationsProgress.FreeRelations;
			SquareFinder squareRootFinder = new SquareFinder(gnfs);

			int freeRelationIndex = 0;
			bool solutionFound = false;

			// Below randomly selects a solution set to try and find a square root of the polynomial in.
			while (!solutionFound)
			{
				if (cancelToken.IsCancellationRequested) { return solutionFound; }

				// Each time this step is stopped and restarted, it will try a different solution set.
				// Previous used sets are tracked with the List<int> triedFreeRelationIndices
				if (triedFreeRelationIndices.Count == freeRelations.Count) // If we have exhausted our solution sets, alert the user. Number wont factor for some reason.
				{
					gnfs.LogMessage("ERROR: ALL RELATION SETS HAVE BEEN TRIED...?");
					gnfs.LogMessage($"If the number of solution sets ({freeRelations.Count}) is low, you may need to sieve some more and then re-run the matrix solving step.");
					gnfs.LogMessage("If there are many solution sets, and you have tried them all without finding non-trivial factors, then something is wrong...");
					gnfs.LogMessage();
					break;
				}

				do
				{
					// Below randomly selects a solution set to try and find a square root of the polynomial in.
					freeRelationIndex = StaticRandom.Next(0, freeRelations.Count);
				}
				while (triedFreeRelationIndices.Contains(freeRelationIndex));

				triedFreeRelationIndices.Add(freeRelationIndex); // Add current selection to our list

				List<Relation> selectedRelationSet = freeRelations[freeRelationIndex]; // Get the solution set

				gnfs.LogMessage();
				gnfs.LogMessage($"Selected solution set index # {freeRelationIndex + 1}");
				gnfs.LogMessage();
				gnfs.LogMessage("Calculating Rational Square Root β ∈ ℤ[θ] ...");
				gnfs.LogMessage();
				squareRootFinder.CalculateRationalSide(cancelToken, selectedRelationSet);

				if (cancelToken.IsCancellationRequested) { gnfs.LogMessage("Abort: Task canceled by user!"); break; }

				gnfs.LogMessage("SquareFinder.CalculateRationalSide() Completed.");
				gnfs.LogMessage();
				gnfs.LogMessage("Calculating Algebraic Square Root...");
				gnfs.LogMessage("                    y ∈ ℤ ...");
				gnfs.LogMessage("δ in a finite field 𝔽ᵨ(θᵨ) ...");
				gnfs.LogMessage();

				Tuple<BigInteger, BigInteger> foundFactors = squareRootFinder.CalculateAlgebraicSide(cancelToken);

				if (cancelToken.IsCancellationRequested) { gnfs.LogMessage("Abort: Task canceled by user!"); break; }

				gnfs.LogMessage("SquareFinder.CalculateAlgebraicSide() Completed.");

				gnfs.LogMessage();
				gnfs.LogMessage($"{squareRootFinder.AlgebraicSquareRootResidue}² ≡ {squareRootFinder.RationalSquareRootResidue}² (mod {squareRootFinder.N})");
				gnfs.LogMessage();

				BigInteger P = foundFactors.Item1;
				BigInteger Q = foundFactors.Item2;

				bool nonTrivialFactorsFound = (P != 1 || Q != 1);
				if (nonTrivialFactorsFound)
				{
					solutionFound = gnfs.SetFactorizationSolution(P, Q);

					gnfs.LogMessage($"Selected solution set index # {freeRelationIndex + 1}");
					gnfs.LogMessage();

					if (solutionFound)
					{
						gnfs.LogMessage("NON-TRIVIAL FACTORS FOUND!");
						gnfs.LogMessage();
						gnfs.LogMessage(squareRootFinder.ToString());
						gnfs.LogMessage();
						gnfs.LogMessage();
						gnfs.LogMessage(gnfs.Factorization.ToString());
						gnfs.LogMessage();
					}
					break;
				}
				else if (cancelToken.IsCancellationRequested)
				{
					gnfs.LogMessage("Abort: Task canceled by user!");
					break;
				}
				else
				{
					gnfs.LogMessage();
					gnfs.LogMessage("Unable to locate a square root in solution set!");
					gnfs.LogMessage();
					gnfs.LogMessage("Trying a different solution set...");
					gnfs.LogMessage();
				}
			}

			return solutionFound;
		}

		public void CalculateRationalSide(CancellationToken cancelToken, List<Relation> relations)
		{
			relationsSet = relations;
			rationalNorms = relationsSet.Select(rel => rel.RationalNorm).ToList();

			CountDictionary rationalSquareFactorization = new CountDictionary();
			foreach (var rel in relationsSet)
			{
				rationalSquareFactorization.Combine(rel.RationalFactorization);
			}

			string rationalSquareFactorizationString = rationalSquareFactorization.FormatStringAsFactorization();

			LogFunction.Invoke("");
			LogFunction.Invoke("Rational Square Dependency:");
			LogFunction.Invoke(rationalSquareFactorizationString);

			if (cancelToken.IsCancellationRequested) { return; }

			RationalProduct = rationalNorms.Product();

			LogFunction.Invoke("");
			LogFunction.Invoke($"δᵣ = {RationalProduct} = {string.Join(" * ", rationalNorms)}");

			BigInteger RationalProductSquareRoot = RationalProduct.SquareRoot();

			var product = PolynomialDerivativeValue * RationalProductSquareRoot;

			RationalSquareRootResidue = product.Mod(N);

			LogFunction.Invoke("");
			LogFunction.Invoke($"δᵣ = {RationalProductSquareRoot}^2 = {RationalProduct}");
			LogFunction.Invoke($"χ  = {RationalSquareRootResidue} ≡ {PolynomialDerivativeValue} * {RationalProductSquareRoot} (mod {N})");
			LogFunction.Invoke("");

			IsRationalSquare = RationalProduct.IsSquare();
			if (!IsRationalSquare) // This is an error in implementation. This should never happen, and so must be a bug
			{
				throw new Exception($"{nameof(IsRationalSquare)} evaluated to false. This is a sign that there is a bug in the implementation, as this should never be the case if the algorithm has been correctly implemented.");
			}
		}

		public Tuple<BigInteger, BigInteger> CalculateAlgebraicSide(CancellationToken cancelToken)
		{
			RootsOfS.AddRange(relationsSet.Select(rel => new Tuple<BigInteger, BigInteger>(rel.A, rel.B)));

			if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); }

			PolynomialRingElements = new List<Polynomial>();
			foreach (Relation rel in relationsSet)
			{
				// poly(x) = A + (B * x)
				Polynomial newPoly =
					new Polynomial(
						new Term[]
						{
							new Term( rel.B, 1),
							new Term( rel.A, 0)
						}
					);

				PolynomialRingElements.Add(newPoly);
			}

			if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); }

			PolynomialRing = Polynomial.Product(PolynomialRingElements);
			Polynomial PolynomialRingInField = Polynomial.Field.Modulus(PolynomialRing, MonicPolynomial);


			LogFunction.Invoke("");
			LogFunction.Invoke($"∏ Sᵢ = {PolynomialRing}");
			LogFunction.Invoke("");
			LogFunction.Invoke($"∏ Sᵢ = {PolynomialRingInField}");
			LogFunction.Invoke(" in ℤ");
			LogFunction.Invoke("");

			if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); }

			// Multiply the product of the polynomial elements by f'(x)^2
			// This will guarantee that the square root of product of polynomials
			// is an element of the number field defined by the algebraic polynomial.
			TotalS = Polynomial.Multiply(PolynomialRing, MonicPolynomialDerivativeSquared);
			S = Polynomial.Field.Modulus(TotalS, MonicPolynomial);

			LogFunction.Invoke("");
			LogFunction.Invoke($"δᵨ = {TotalS}");
			LogFunction.Invoke($"δᵨ = {S}");
			LogFunction.Invoke(" in ℤ");

			bool solutionFound = false;

			int degree = MonicPolynomial.Degree;
			Polynomial f = MonicPolynomial;// gnfs.CurrentPolynomial;

			BigInteger lastP = gnfs.QuadraticFactorPairCollection.Last().P; //quadraticPrimes.First(); //BigInteger.Max(fromRoot, fromQuadraticFactorPairs); //N / N.ToString().Length; //((N * 3) + 1).NthRoot(3); //gnfs.QFB.Select(fp => fp.P).Max();
			lastP = PrimeFactory.GetNextPrime(lastP + 1);

			List<BigInteger> primes = new List<BigInteger>();
			List<BigInteger> values = new List<BigInteger>();

			int attempts = 7;
			while (!solutionFound && attempts > 0)
			{
				if (primes.Count > 0 && values.Count > 0)
				{
					primes.Clear();
					values.Clear();
				}

				do
				{
					if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); }

					lastP = PrimeFactory.GetNextPrime(lastP + 1);

					Polynomial g = Polynomial.Parse($"X^{lastP} - X");
					Polynomial h = FiniteFieldArithmetic.ModMod(g, f, lastP);

					Polynomial gcd = Polynomial.Field.GCD(h, f, lastP);

					bool isIrreducible = gcd.CompareTo(Polynomial.One) == 0;
					if (!isIrreducible)
					{
						continue;
					}

					primes.Add(lastP);
				}
				while (primes.Count < degree);

				if (primes.Count > degree)
				{
					primes.Remove(primes.First());
					values.Remove(values.First());
				}

				BigInteger primeProduct = primes.Product();

				if (primeProduct < N)
				{
					continue;
				}

				if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); ; }

				bool takeInverse = false;
				foreach (BigInteger p in primes)
				{
					Polynomial choosenPoly = FiniteFieldArithmetic.SquareRoot(S, f, p, degree, gnfs.PolynomialBase);
					BigInteger choosenX;

					//if (takeInverse)
					//{
					//	Polynomial inverse = ModularInverse(choosenPoly, p);
					//	BigInteger inverseEval = inverse.Evaluate(gnfs.PolynomialBase);
					//	BigInteger inverseX = inverseEval.Mod(p);
					//
					//	choosenPoly = inverse;
					//	choosenX = inverseX;
					//}
					//else
					//{
					BigInteger eval = choosenPoly.Evaluate(gnfs.PolynomialBase);
					BigInteger x = eval.Mod(p);

					choosenX = x;
					//}

					values.Add(choosenX);

					LogFunction.Invoke("");
					LogFunction.Invoke($" β = {choosenPoly}");
					LogFunction.Invoke($"xi = {choosenX}");
					LogFunction.Invoke($" p = {p}");
					LogFunction.Invoke($"{primeProduct / p}");
					LogFunction.Invoke("");

					takeInverse = !takeInverse;
				}

				BigInteger commonModulus = Polynomial.Algorithms.ChineseRemainderTheorem(primes.ToArray(), values.ToArray()); //FiniteFieldArithmetic.ChineseRemainder(primes, values);
				AlgebraicSquareRootResidue = commonModulus.Mod(N);

				LogFunction.Invoke("");

				int index = -1;
				while ((++index) < primes.Count)
				{
					var tp = primes[index];
					var tv = values[index];

					LogFunction.Invoke($"{tp} ≡ {tv} (mod {AlgebraicSquareRootResidue})");
				}



				LogFunction.Invoke("");
				LogFunction.Invoke($"γ = {AlgebraicSquareRootResidue}"); // δ mod N 

				BigInteger algebraicSquareRoot = 1;

				BigInteger min;
				BigInteger max;
				BigInteger A;
				BigInteger B;
				BigInteger U;
				BigInteger V;
				BigInteger P = 0;
				BigInteger Q;

				if (cancelToken.IsCancellationRequested) { return new Tuple<BigInteger, BigInteger>(1, 1); }

				min = BigInteger.Min(RationalSquareRootResidue, AlgebraicSquareRootResidue);
				max = BigInteger.Max(RationalSquareRootResidue, AlgebraicSquareRootResidue);

				A = max + min;
				B = max - min;

				U = GCD.FindGCD(N, A);
				V = GCD.FindGCD(N, B);

				if (U > 1 && U != N)
				{
					P = U;
					solutionFound = true;
				}
				else if (V > 1 && V != N)
				{
					P = V;
					solutionFound = true;
				}

				if (solutionFound)
				{
					BigInteger rem;
					BigInteger other = BigInteger.DivRem(N, P, out rem);

					if (rem != 0)
					{
						solutionFound = false;
					}
					else
					{
						Q = other;
						AlgebraicResults = values;
						//AlgebraicSquareRootResidue = AlgebraicSquareRootResidue;
						AlgebraicPrimes = primes;

						return new Tuple<BigInteger, BigInteger>(P, Q);
					}
				}

				if (!solutionFound)
				{
					GNFS.LogFunction($"No solution found amongst the algebraic square roots {{ {string.Join(", ", values.Select(v => v.ToString()))} }} mod primes {{ {string.Join(", ", primes.Select(p => p.ToString()))} }}");

					attempts--;
				}
			}

			return new Tuple<BigInteger, BigInteger>(1, 1);
		}

		private static Tuple<BigInteger, BigInteger> AlgebraicSquareRoot(Polynomial f, BigInteger m, int degree, Polynomial dd, BigInteger p)
		{
			Polynomial startPolynomial = Polynomial.Field.Modulus(dd, p);
			Polynomial startInversePolynomial = ModularInverse(startPolynomial, p);

			Polynomial startSquared1 = FiniteFieldArithmetic.ModMod(Polynomial.Square(startPolynomial), f, p);
			Polynomial startSquared2 = FiniteFieldArithmetic.ModMod(Polynomial.Square(startInversePolynomial), f, p);

			Polynomial resultPoly1 = FiniteFieldArithmetic.SquareRoot(startPolynomial, f, p, degree, m);
			Polynomial resultPoly2 = ModularInverse(resultPoly1, p);

			Polynomial resultSquared1 = FiniteFieldArithmetic.ModMod(Polynomial.Square(resultPoly1), f, p);
			Polynomial resultSquared2 = FiniteFieldArithmetic.ModMod(Polynomial.Square(resultPoly2), f, p);

			bool bothResultsAgree = (resultSquared1.CompareTo(resultSquared2) == 0);

			bool resultSquaredEqualsInput1 = (startPolynomial.CompareTo(resultSquared1) == 0);
			bool resultSquaredEqualsInput2 = (startInversePolynomial.CompareTo(resultSquared1) == 0);

			BigInteger result1 = resultPoly1.Evaluate(m).Mod(p);
			BigInteger result2 = resultPoly2.Evaluate(m).Mod(p);

			BigInteger inversePrime = p - result1;
			bool testEvaluationsAreModularInverses = inversePrime == result2;

			if (bothResultsAgree && testEvaluationsAreModularInverses)
			{
				return new Tuple<BigInteger, BigInteger>(BigInteger.Min(result1, result2), BigInteger.Max(result1, result2));
			}

			return new Tuple<BigInteger, BigInteger>(BigInteger.Zero, BigInteger.Zero);
		}

		private static Polynomial ModularInverse(Polynomial poly, BigInteger mod)
		{
			return new Polynomial(Term.GetTerms(poly.Terms.Select(trm => (mod - trm.CoEfficient).Mod(mod)).ToArray()));
		}

		public override string ToString()
		{
			StringBuilder result = new StringBuilder();

			result.AppendLine("Polynomial ring:");
			result.AppendLine($"({string.Join(") * (", PolynomialRingElements.Select(ply => ply.ToString()))})");
			result.AppendLine();
			result.AppendLine($"∏ Sᵢ =");
			result.AppendLine($"{PolynomialRing}");
			result.AppendLine();
			result.AppendLine($"ƒ         = {gnfs.CurrentPolynomial}");
			result.AppendLine($"ƒ(m)      = {MonicPolynomial}");
			result.AppendLine($"ƒ'(m)     = {MonicPolynomialDerivative}");
			result.AppendLine($"ƒ'(m)^2   = {MonicPolynomialDerivativeSquared}");
			result.AppendLine();
			result.AppendLine($"∏ Sᵢ(m)  *  ƒ'(m)² =");
			result.AppendLine($"{TotalS}");
			result.AppendLine();
			result.AppendLine($"∏ Sᵢ(m)  *  ƒ'(m)² (mod ƒ) =");
			result.AppendLine($"{S}");
			result.AppendLine();
			result.AppendLine();
			result.AppendLine("Square finder, Rational:");
			result.AppendLine($"γ² = √(  Sᵣ(m)  *  ƒ'(m)²  )");
			result.AppendLine($"γ² = √( {RationalProduct} * {PolynomialDerivativeValueSquared} )");
			result.AppendLine($"γ² = √( {RationalSquare} )");
			result.AppendLine($"IsRationalSquare  ? {IsRationalSquare}");
			result.AppendLine($"γ  =    {RationalSquareRootResidue} mod N"); // δ mod N 
			result.AppendLine($"IsRationalIrreducible  ? {IsRationalIrreducible}");
			result.AppendLine();
			result.AppendLine();
			result.AppendLine("Square finder, Algebraic:");
			result.AppendLine($"    Sₐ(m) * ƒ'(m)  =  {AlgebraicProduct} * {PolynomialDerivativeValue}");
			result.AppendLine($"    Sₐ(m) * ƒ'(m)  =  {AlgebraicSquare}");
			result.AppendLine($"IsAlgebraicSquare ? {IsAlgebraicSquare}");
			result.AppendLine($"χ = Sₐ(m) * ƒ'(m) mod N = {AlgebraicSquareRootResidue}");
			result.AppendLine($"IsAlgebraicIrreducible ? {IsAlgebraicIrreducible}");
			result.AppendLine();
			result.AppendLine($"X² / ƒ(m) = {AlgebraicProductModF}  IsSquare? {AlgebraicProductModF.IsSquare()}");
			result.AppendLine($"S (x)       = {AlgebraicSquareResidue}  IsSquare? {AlgebraicSquareResidue.IsSquare()}");
			result.AppendLine($"AlgebraicResults:");
			result.AppendLine($"{AlgebraicResults.FormatString(false)}");
			result.AppendLine();
			result.AppendLine();

			result.AppendLine("Primes:");
			result.AppendLine($"{string.Join(" * ", AlgebraicPrimes)}"); // .RelationsSet.Select(rel => rel.B).Distinct().OrderBy(relB => relB))
			result.AppendLine();
			result.AppendLine();
			//result.AppendLine("Roots of S(x):");
			//result.AppendLine($"{{{string.Join(", ", RootsOfS.Select(tup => (tup.Item2 > 1) ? $"{tup.Item1}/{tup.Item2}" : $"{tup.Item1}"))}}}");
			//result.AppendLine();
			//result.AppendLine();
			//result.AppendLine($"∏(a + mb) = {squareRootFinder.RationalProduct}");
			//result.AppendLine($"∏ƒ(a/b)   = {squareRootFinder.AlgebraicProduct}");
			//result.AppendLine();

			BigInteger min = BigInteger.Min(RationalSquareRootResidue, AlgebraicSquareRootResidue);
			BigInteger max = BigInteger.Max(RationalSquareRootResidue, AlgebraicSquareRootResidue);

			BigInteger add = max + min;
			BigInteger sub = max - min;

			BigInteger gcdAdd = GCD.FindGCD(N, add);
			BigInteger gcdSub = GCD.FindGCD(N, sub);

			BigInteger answer = BigInteger.Max(gcdAdd, gcdSub);


			result.AppendLine();
			result.AppendLine($"GCD(N, γ+χ) = {gcdAdd}");
			result.AppendLine($"GCD(N, γ-χ) = {gcdSub}");
			result.AppendLine();
			result.AppendLine($"Solution? {(answer != 1).ToString().ToUpper()}");

			if (answer != 1)
			{
				result.AppendLine();
				result.AppendLine();
				result.AppendLine("*********************");
				result.AppendLine();
				result.AppendLine($" SOLUTION = {answer} ");
				result.AppendLine();
				result.AppendLine("*********************");
				result.AppendLine();
				result.AppendLine();
			}

			result.AppendLine();

			return result.ToString();
		}
	}
}

