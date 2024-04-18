using System;
using System.Linq;
using System.Numerics;
using System.Collections;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Collections.Concurrent;

namespace GNFSCore.IntegerMath
{
	public static partial class FactorizationFactory
	{
		private static BigInteger[] primeCheckBases = new BigInteger[] { 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47 };
		public static bool IsProbablePrime(BigInteger input)
		{
			if (input == 2 || input == 3)
			{
				return true;
			}
			if (input < 2 || input % 2 == 0)
			{
				return false;
			}

			BigInteger d = input - 1;
			int s = 0;

			while (d % 2 == 0)
			{
				d /= 2;
				s += 1;
			}

			foreach (BigInteger a in primeCheckBases)
			{
				BigInteger x = BigInteger.ModPow(a, d, input);
				if (x == 1 || x == input - 1)
				{
					continue;
				}

				for (int r = 1; r < s; r++)
				{
					x = BigInteger.ModPow(x, 2, input);
					if (x == 1)
					{
						return false;
					}
					if (x == input - 1)
					{
						break;
					}
				}

				if (x != input - 1)
				{
					return false;
				}
			}

			return true;
		}
	}
}


using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace GNFSCore.IntegerMath
{
	public static class Combinatorics
	{
		/// <summary>
		/// Returns the Cartesian product of two or more lists
		/// </summary>
		public static List<List<T>> CartesianProduct<T>(IEnumerable<IEnumerable<T>> sequences)
		{
			IEnumerable<IEnumerable<T>> empty = new[] { Enumerable.Empty<T>() };
			return sequences.Aggregate
					(
						empty,
						(first, second) =>
							from a in first
							from b in second
							select a.Concat(new[] { b })
					)
					.Select(lst => lst.ToList())
					.ToList();
		}
	}
}

using System;
using System.Linq;
using System.Numerics;
using System.Collections;
using System.Collections.Generic;
using GNFSCore.Core;

namespace GNFSCore.IntegerMath
{
	public class FastPrimeSieve : IEnumerable<BigInteger>
	{
		private static readonly uint PageSize; // L1 CPU cache size in bytes
		private static readonly uint BufferBits;
		private static readonly uint BufferBitsNext;

		static FastPrimeSieve()
		{
			uint cacheSize = 393216;
			List<uint> cacheSizes = CPUInfo.GetCacheSizes(CPUInfo.CacheLevel.Level1);
			if (cacheSizes.Any())
			{
				cacheSize = cacheSizes.First() * 1024;
			}

			PageSize = cacheSize; // L1 CPU cache size in bytes
			BufferBits = PageSize * 8; // in bits
			BufferBitsNext = BufferBits * 2;
		}

		public static IEnumerable<BigInteger> GetRange(BigInteger floor, BigInteger ceiling)
		{
			FastPrimeSieve primesPaged = new FastPrimeSieve();
			IEnumerator<BigInteger> enumerator = primesPaged.GetEnumerator();

			while (enumerator.MoveNext())
			{
				if (enumerator.Current >= floor)
				{
					break;
				}
			}

			do
			{
				if (enumerator.Current > ceiling)
				{
					break;
				}
				yield return enumerator.Current;
			}
			while (enumerator.MoveNext());

			yield break;
		}

		public IEnumerator<BigInteger> GetEnumerator()
		{
			return Iterator();
		}

		IEnumerator IEnumerable.GetEnumerator()
		{
			return (IEnumerator)GetEnumerator();
		}

		private static IEnumerator<BigInteger> Iterator()
		{
			IEnumerator<BigInteger> basePrimes = null;
			List<uint> basePrimesArray = new List<uint>();
			uint[] cullBuffer = new uint[PageSize / 4]; // 4 byte words

			yield return 2;

			for (var low = (BigInteger)0; ; low += BufferBits)
			{
				for (var bottomItem = 0; ; ++bottomItem)
				{
					if (bottomItem < 1)
					{
						if (bottomItem < 0)
						{
							bottomItem = 0;
							yield return 2;
						}

						BigInteger next = 3 + low + low + BufferBitsNext;
						if (low <= 0)
						{
							// cull very first page
							for (int i = 0, sqr = 9, p = 3; sqr < next; i++, p += 2, sqr = p * p)
							{
								if ((cullBuffer[i >> 5] & (1 << (i & 31))) == 0)
								{
									for (int j = (sqr - 3) >> 1; j < BufferBits; j += p)
									{
										cullBuffer[j >> 5] |= 1u << j;
									}
								}
							}
						}
						else
						{
							// Cull for the rest of the pages
							Array.Clear(cullBuffer, 0, cullBuffer.Length);

							if (basePrimesArray.Count == 0)
							{
								// Init second base primes stream
								basePrimes = Iterator();
								basePrimes.MoveNext();
								basePrimes.MoveNext();
								basePrimesArray.Add((uint)basePrimes.Current); // Add 3 to base primes array
								basePrimes.MoveNext();
							}

							// Make sure basePrimesArray contains enough base primes...
							for (BigInteger p = basePrimesArray[basePrimesArray.Count - 1], square = p * p; square < next;)
							{
								p = basePrimes.Current;
								basePrimes.MoveNext();
								square = p * p;
								basePrimesArray.Add((uint)p);
							}

							for (int i = 0, limit = basePrimesArray.Count - 1; i < limit; i++)
							{
								var p = (BigInteger)basePrimesArray[i];
								var start = (p * p - 3) >> 1;

								// adjust start index based on page lower limit...
								if (start >= low)
								{
									start -= low;
								}
								else
								{
									var r = (low - start) % p;
									start = (r != 0) ? p - r : 0;
								}
								for (var j = (uint)start; j < BufferBits; j += (uint)p)
								{
									cullBuffer[j >> 5] |= 1u << ((int)j);
								}
							}
						}
					}

					while (bottomItem < BufferBits && (cullBuffer[bottomItem >> 5] & (1 << (bottomItem & 31))) != 0)
					{
						++bottomItem;
					}

					if (bottomItem < BufferBits)
					{
						var result = 3 + (((BigInteger)bottomItem + low) << 1);
						yield return result;
					}
					else break; // outer loop for next page segment...
				}
			}
		}
	}
}

using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Text;
using System.Threading.Tasks;

namespace GNFSCore.IntegerMath
{
	public static class GCD
	{
		public static BigInteger FindLCM(IEnumerable<BigInteger> numbers)
		{
			return FindLCM(numbers.ToArray());
		}

		public static BigInteger FindLCM(params BigInteger[] numbers)
		{
			return numbers.Aggregate(FindLCM);
		}

		public static BigInteger FindLCM(BigInteger left, BigInteger right)
		{
			BigInteger absValue1 = BigInteger.Abs(left);
			BigInteger absValue2 = BigInteger.Abs(right);
			return (absValue1 * absValue2) / FindGCD(absValue1, absValue2);
		}

		public static BigInteger FindGCD(IEnumerable<BigInteger> numbers)
		{
			return FindGCD(numbers.ToArray());
		}

		public static BigInteger FindGCD(params BigInteger[] numbers)
		{
			return numbers.Aggregate(FindGCD);
		}

		public static BigInteger FindGCD(BigInteger left, BigInteger right)
		{
			return BigInteger.GreatestCommonDivisor(left, right);
		}

		public static bool AreCoprime(params BigInteger[] numbers)
		{
			return (FindGCD(numbers.ToArray()) == 1);
		}
	}
}

using System;
using System.Linq;
using System.Text;
using System.Numerics;
using System.Threading.Tasks;
using System.Collections.Generic;

namespace GNFSCore.IntegerMath
{
	public static class Legendre
	{
		/// <summary>
		/// Legendre Symbol returns 1 for a (nonzero) quadratic residue mod p, -1 for a non-quadratic residue (non-residue), or 0 on zero.
		/// </summary>		
		public static int Symbol(BigInteger a, BigInteger p)
		{
			if (p < 2) { throw new ArgumentOutOfRangeException(nameof(p), $"Parameter '{nameof(p)}' must not be < 2, but you have supplied: {p}"); }
			if (a == 0) { return 0; }
			if (a == 1) { return 1; }

			int result;
			if (a.Mod(2) == 0)
			{
				result = Symbol(a >> 2, p); // >> right shift == /2
				if (((p * p - 1) & 8) != 0) // instead of dividing by 8, shift the mask bit
				{
					result = -result;
				}
			}
			else
			{
				result = Symbol(p.Mod(a), a);
				if (((a - 1) * (p - 1) & 4) != 0) // instead of dividing by 4, shift the mask bit
				{
					result = -result;
				}
			}
			return result;
		}

		/// <summary>
		///  Find r such that (r | m) = goal, where  (r | m) is the Legendre symbol, and m = modulus
		/// </summary>
		public static BigInteger SymbolSearch(BigInteger start, BigInteger modulus, BigInteger goal)
		{
			if (goal != -1 && goal != 0 && goal != 1)
			{
				throw new Exception($"Parameter '{nameof(goal)}' may only be -1, 0 or 1. It was {goal}.");
			}

			BigInteger counter = start;
			BigInteger max = counter + modulus + 1;
			do
			{
				if (Symbol(counter, modulus) == goal)
				{
					return counter;
				}
				counter++;
			}
			while (counter <= max);

			//return counter;
			throw new Exception("Legendre symbol matching criteria not found.");
		}
	}

}
using System;
using System.Linq;
using System.Numerics;
using ExtendedArithmetic;
using ExtendedNumerics;

namespace GNFSCore.Factors
{
	using Interfaces;

	public static class Normal
	{
		/// <summary>
		///  a + bm
		/// </summary>
		/// <param name="polynomialBase">Base m of f(m) = N</param>
		/// <returns></returns>
		public static BigInteger Rational(BigInteger a, BigInteger b, BigInteger polynomialBase)
		{
			return BigInteger.Add(a, BigInteger.Multiply(b, polynomialBase));
		}

		/// <summary>
		/// a - bm
		/// </summary>
		public static BigInteger RationalSubtract(BigInteger a, BigInteger b, BigInteger polynomialBase)
		{
			return BigInteger.Subtract(a, BigInteger.Multiply(b, polynomialBase));
		}

		/// <summary>
		/// ƒ(b) ≡ 0 (mod a)
		/// 
		/// Calculated as:
		/// ƒ(-a/b) * -b^deg
		/// </summary>
		/// <param name="a">Divisor in the equation ƒ(b) ≡ 0 (mod a)</param>
		/// <param name="b">A root of f(x)</param>
		/// <param name="poly">Base m of f(m) = N</param>
		/// <returns></returns>
		public static BigInteger Algebraic(BigInteger a, BigInteger b, Polynomial poly)
		{
			BigRational aD = (BigRational)a;
			BigRational bD = (BigRational)b;
			BigRational ab = BigRational.Negate(aD) / bD;

			BigRational left = PolynomialEvaluate_BigRational(poly, ab);
			BigInteger right = BigInteger.Pow(BigInteger.Negate(b), poly.Degree);

			BigRational product = right * left;

			Fraction fractionalPart = product.FractionalPart;
			if (fractionalPart != Fraction.Zero)
			{
				GNFS.LogFunction($"{nameof(Algebraic)} failed to result in an integer. This shouldn't happen.");
			}

			BigInteger result = product.WholePart;
			return result;
		}

		private static BigRational PolynomialEvaluate_BigRational(Polynomial polynomial, BigRational indeterminateValue)
		{
			int num = polynomial.Degree;

			BigRational result = (BigRational)polynomial[num];
			while (--num >= 0)
			{
				result *= indeterminateValue;
				result += (BigRational)polynomial[num];
			}

			return result;
		}
	}
}
using System;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Collections.Generic;

namespace GNFSCore.IntegerMath
{
	using System.Numerics;

	public static class PrimeFactory
	{
		private static BigInteger MaxValue = 10;

		private static int primesCount;
		private static BigInteger primesLast;
		private static List<BigInteger> primes = new List<BigInteger>() { 2, 3, 5, 7, 11, 13 };

		static PrimeFactory()
		{
			SetPrimes();
		}

		private static void SetPrimes()
		{
			primes = FastPrimeSieve.GetRange(2, (Int32)MaxValue).ToList();
			primesCount = primes.Count;
			primesLast = primes.Last();
		}

		public static IEnumerable<BigInteger> GetPrimeEnumerator(int startIndex = 0, int stopIndex = -1)
		{
			int index = startIndex;
			int maxIndex = stopIndex > 0 ? stopIndex : primesCount - 1;
			while (index < maxIndex)
			{
				yield return primes[index];
				index++;
			}
			yield break;
		}

		public static void IncreaseMaxValue(BigInteger newMaxValue)
		{
			// Increase bound
			BigInteger temp = BigInteger.Max(newMaxValue + 1000, MaxValue + 100000 /*MaxValue*/);
			MaxValue = BigInteger.Min(temp, (Int32.MaxValue - 1));
			SetPrimes();
		}

		public static int GetIndexFromValue(BigInteger value)
		{
			if (value == -1)
			{
				return -1;
			}
			if (primesLast < value)
			{
				IncreaseMaxValue(value);
			}

			BigInteger primeValue = primes.First(p => p >= value);

			int index = primes.IndexOf(primeValue) + 1;
			return index;
		}

		public static BigInteger GetApproximateValueFromIndex(UInt64 n)
		{
			if (n < 6)
			{
				return primes[(int)n];
			}

			double fn = (double)n;
			double flogn = Math.Log(n);
			double flog2n = Math.Log(flogn);

			double upper;

			if (n >= 688383)    /* Dusart 2010 page 2 */
			{
				upper = fn * (flogn + flog2n - 1.0 + ((flog2n - 2.00) / flogn));
			}
			else if (n >= 178974)    /* Dusart 2010 page 7 */
			{
				upper = fn * (flogn + flog2n - 1.0 + ((flog2n - 1.95) / flogn));
			}
			else if (n >= 39017)    /* Dusart 1999 page 14 */
			{
				upper = fn * (flogn + flog2n - 0.9484);
			}
			else                    /* Modified from Robin 1983 for 6-39016 _only_ */
			{
				upper = fn * (flogn + 0.6000 * flog2n);
			}

			if (upper >= (double)UInt64.MaxValue)
			{
				throw new OverflowException($"{upper} > {UInt64.MaxValue}");
			}

			return new BigInteger((UInt64)Math.Ceiling(upper));
		}

		public static IEnumerable<BigInteger> GetPrimesFrom(BigInteger minValue)
		{
			return GetPrimeEnumerator(GetIndexFromValue(minValue));
		}

		public static IEnumerable<BigInteger> GetPrimesTo(BigInteger maxValue)
		{
			if (primesLast < maxValue)
			{
				IncreaseMaxValue(maxValue);
			}
			return GetPrimeEnumerator(0).TakeWhile(p => p < maxValue);
		}

		public static bool IsPrime(BigInteger value)
		{
			return primes.Contains(BigInteger.Abs(value));
		}

		public static BigInteger GetNextPrime(BigInteger fromValue)
		{
			BigInteger result = fromValue + 1;

			if (result.IsEven)
			{
				result += 1;
			}

			while (!FactorizationFactory.IsProbablePrime(result))
			{
				result += 2;
			}

			return result;
		}
	}
}
using System;
using System.Linq;
using System.Numerics;
using System.Text;
using System.Threading.Tasks;
using System.Collections.Generic;

namespace GNFSCore.IntegerMath
{
	using Factors;

	public class QuadraticResidue
	{
		// a^(p-1)/2 ≡ 1 (mod p)
		public static bool IsQuadraticResidue(BigInteger a, BigInteger p)
		{
			BigInteger quotient = BigInteger.Divide(p - 1, 2);
			BigInteger modPow = BigInteger.ModPow(a, quotient, p);

			return modPow.IsOne;
		}

		public static bool GetQuadraticCharacter(Relation rel, FactorPair quadraticFactor)
		{
			BigInteger ab = rel.A + rel.B;
			BigInteger abp = BigInteger.Abs(BigInteger.Multiply(ab, quadraticFactor.P));

			int legendreSymbol = Legendre.Symbol(abp, quadraticFactor.R);
			return (legendreSymbol != 1);
		}
	}
}
