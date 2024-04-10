using System;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore
{
	using Interfaces;

	[JsonDictionary]
	public class CountDictionary : SortedDictionary<BigInteger, BigInteger>, ICloneable<Dictionary<BigInteger, BigInteger>>
	{
		public CountDictionary()
			: base(Comparer<BigInteger>.Create(BigInteger.Compare))
		{			
		}

		public void Add(BigInteger key)
		{
			this.AddSafe(key, 1);
		}
		private void AddSafe(BigInteger key, BigInteger value)
		{
			if (!ContainsKey(key)) { this.Add(key, value); }
			else { this[key] += value; }
		}

		public void Combine(CountDictionary dictionary)
		{
			foreach (var kvp in dictionary)
			{
				AddSafe(kvp.Key, kvp.Value);
			}
		}

		public Dictionary<BigInteger, BigInteger> ToDictionary()
		{
			return this.ToDictionary(kvp => kvp.Key, kvp => kvp.Value);
		}

		public Dictionary<BigInteger, BigInteger> Clone()
		{
			return this.ToDictionary();
		}

		#region String Formatting

		public override string ToString()
		{
			//Order();

			StringBuilder result = new StringBuilder();
			result.AppendLine("{");
			foreach (KeyValuePair<BigInteger, BigInteger> kvp in this)
			{
				result.Append('\t');
				result.Append(kvp.Key.ToString().PadLeft(5));
				result.Append(":\t");
				result.AppendLine(kvp.Value.ToString().PadLeft(5));
			}
			result.Append("}");

			return result.ToString();
		}

		public string FormatStringAsFactorization()
		{
			//Order();
			StringBuilder result = new StringBuilder();
			result.Append(
				" -> {\t" +
				string.Join(" * ", this.Select(kvp => $"{ kvp.Key}^{ kvp.Value}")) +
				"\t};"
				);
			return result.ToString();
		}

		#endregion
	}

}

using System;
using System.Linq;
using System.Management;
using System.Collections.Generic;

namespace GNFSCore.Core
{
	public static class CPUInfo
	{
		public static List<uint> GetCacheSizes(CacheLevel level)
		{
			ManagementClass mc = new ManagementClass("Win32_CacheMemory");
			ManagementObjectCollection moc = mc.GetInstances();
			List<uint> cacheSizes = new List<uint>(moc.Count);

			cacheSizes.AddRange(moc
			  .Cast<ManagementObject>()
			  .Where(p => (ushort)(p.Properties["Level"].Value) == (ushort)level)
			  .Select(p => (uint)(p.Properties["MaxCacheSize"].Value)));

			return cacheSizes;
		}

		public enum CacheLevel : ushort
		{
			Level1 = 3,
			Level2 = 4,
			Level3 = 5,
		}
	}
}

using System;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore
{
	public class DirectoryLocations
	{
		private const int showDigits = 22;
		private const string elipse = "[...]";

		private static string baseDirectory = "GNFS";
		private static string parametersFilename = "GNFS.json";
		private static string rationalFactorPairFilename = $"{nameof(GNFS.RationalFactorPairCollection)}.json";
		private static string algebraicFactorPairFilename = $"{nameof(GNFS.AlgebraicFactorPairCollection)}.json";
		private static string quadraticFactorPairFilename = $"{nameof(GNFS.QuadraticFactorPairCollection)}.json";
		private static string smoothRelationsFilename = $"{nameof(RelationContainer.SmoothRelations)}.json";
		private static string roughRelationsFilename = $"{nameof(RelationContainer.RoughRelations)}.json";
		private static string freeRelationsFilenameSearchpattern = $"{nameof(RelationContainer.FreeRelations)}_*.json";

		private string _saveDirectory = null;
		private string _rationalFactorPairFilepath = null;
		private string _algebraicFactorPairFilepath = null;
		private string _quadraticFactorPairFilepath = null;
		private string _parametersFilepath = null;
		private string _smoothRelationsFilepath = null;
		private string _roughRelationsFilepath = null;

		public static string SaveFilename { get { return parametersFilename; } }
		public string SaveDirectory { get { return _saveDirectory; } }

		public string GnfsParameters_SaveFile
		{
			get
			{
				if (_parametersFilepath == null)
				{
					_parametersFilepath = Path.Combine(SaveDirectory, parametersFilename);
				}
				return _parametersFilepath;
			}
		}

		public string RationalFactorPair_SaveFile
		{
			get
			{
				if (_rationalFactorPairFilepath == null)
				{
					_rationalFactorPairFilepath = Path.Combine(SaveDirectory, rationalFactorPairFilename);
				}
				return _rationalFactorPairFilepath;
			}
		}

		public string AlgebraicFactorPair_SaveFile
		{
			get
			{
				if (_algebraicFactorPairFilepath == null)
				{
					_algebraicFactorPairFilepath = Path.Combine(SaveDirectory, algebraicFactorPairFilename);
				}
				return _algebraicFactorPairFilepath;
			}
		}

		public string QuadraticFactorPair_SaveFile
		{
			get
			{
				if (_quadraticFactorPairFilepath == null)
				{
					_quadraticFactorPairFilepath = Path.Combine(SaveDirectory, quadraticFactorPairFilename);
				}
				return _quadraticFactorPairFilepath;
			}
		}

		public string SmoothRelations_SaveFile
		{
			get
			{
				if (_smoothRelationsFilepath == null)
				{
					_smoothRelationsFilepath = Path.Combine(SaveDirectory, smoothRelationsFilename);
				}
				return _smoothRelationsFilepath;
			}
		}

		public string RoughRelations_SaveFile
		{
			get
			{
				if (_roughRelationsFilepath == null)
				{
					_roughRelationsFilepath = Path.Combine(SaveDirectory, roughRelationsFilename);
				}
				return _roughRelationsFilepath;
			}
		}

		public string FreeRelations_SearchPattern { get { return freeRelationsFilenameSearchpattern; } }

		public DirectoryLocations(string saveLocation)
		{
			_saveDirectory = saveLocation;
		}

		public DirectoryLocations(BigInteger n)
			: this(GetSaveLocation(n))
		{
		}

		public static void SetBaseDirectory(string path)
		{
			baseDirectory = path;
		}

		public static string GetSaveLocation(BigInteger n)
		{
			string directoryName = GetUniqueNameFromN(n);
			return Path.Combine(baseDirectory, directoryName);
		}

		public static string GetUniqueNameFromN(BigInteger n)
		{
			string result = n.ToString();

			if (result.Length >= (showDigits * 2) + elipse.Length)
			{
				result = result.Substring(0, showDigits) + elipse + result.Substring(result.Length - showDigits, showDigits);
			}

			return result;
		}

		public IEnumerable<string> EnumerateFreeRelationFiles()
		{
			return Directory.EnumerateFiles(SaveDirectory, FreeRelations_SearchPattern);
		}
	}
}


using System;
using System.Linq;
using System.Numerics;
using Newtonsoft.Json;
using System.Collections.Generic;

namespace GNFSCore
{
	public class FactorBase
	{
		public FactorBase()
		{
			RationalFactorBase = new List<BigInteger>();
			AlgebraicFactorBase = new List<BigInteger>();
			QuadraticFactorBase = new List<BigInteger>();
		}

		[JsonProperty]
		public BigInteger RationalFactorBaseMax { get; internal set; }
		[JsonProperty]
		public BigInteger AlgebraicFactorBaseMax { get; internal set; }
		[JsonProperty]
		public BigInteger QuadraticFactorBaseMin { get; internal set; }
		[JsonProperty]
		public BigInteger QuadraticFactorBaseMax { get; internal set; }
		[JsonProperty]
		public int QuadraticBaseCount { get; internal set; }
		[JsonIgnore]
		public IEnumerable<BigInteger> RationalFactorBase { get; internal set; }
		[JsonIgnore]
		public IEnumerable<BigInteger> AlgebraicFactorBase { get; internal set; }
		[JsonIgnore]
		public IEnumerable<BigInteger> QuadraticFactorBase { get; internal set; }
	}
}

using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Numerics;
using Newtonsoft.Json;
using System.Threading;
using System.Collections.Generic;
using System.Runtime.Serialization;
using ExtendedArithmetic;

namespace GNFSCore
{
	using Factors;
	using Interfaces;
	using IntegerMath;

	[DataContract]
	public partial class GNFS
	{
		#region Properties

		[DataMember]
		public BigInteger N { get; set; }

		[DataMember]
		public Solution Factorization { get; private set; }
		[IgnoreDataMember]
		public bool IsFactored { get { return Factorization != null; } }

		public int PolynomialDegree { get; internal set; }
		[DataMember]
		public BigInteger PolynomialBase { get; private set; }

		//[JsonProperty(ItemConverterType = typeof(Serialization.JsonPolynomialConverter))]

		[IgnoreDataMember]
		public List<Polynomial> PolynomialCollection { get; set; }

		[IgnoreDataMember]
		public Polynomial CurrentPolynomial { get; internal set; }

		[DataMember]
		public PolyRelationsSieveProgress CurrentRelationsProgress { get; set; }

		[DataMember]
		public FactorBase PrimeFactorBase { get; set; }

		/// <summary>
		/// Array of (p, m % p)
		/// </summary>
		public FactorPairCollection RationalFactorPairCollection { get; set; }

		/// <summary>
		/// Array of (p, r) where ƒ(r) % p == 0
		/// </summary>
		public FactorPairCollection AlgebraicFactorPairCollection { get; set; }
		public FactorPairCollection QuadraticFactorPairCollection { get; set; }

		public DirectoryLocations SaveLocations { get; internal set; }

		public static LogMessageDelegate LogFunction { get; set; }

		public delegate void LogMessageDelegate(string message);

		#endregion

		#region Constructors 

		public GNFS()
		{
			Factorization = null;
			PrimeFactorBase = new FactorBase();
			PolynomialCollection = new List<Polynomial>();
			RationalFactorPairCollection = new FactorPairCollection();
			AlgebraicFactorPairCollection = new FactorPairCollection();
			QuadraticFactorPairCollection = new FactorPairCollection();
			CurrentRelationsProgress = new PolyRelationsSieveProgress();
		}

		public GNFS(CancellationToken cancelToken, LogMessageDelegate logFunction, BigInteger n, BigInteger polynomialBase, int polyDegree, BigInteger primeBound, int relationQuantity, int relationValueRange, bool createdNewData = false)
			: this()
		{
			LogFunction = logFunction;
			N = n;

			SaveLocations = new DirectoryLocations(N);

			if (createdNewData || !Directory.Exists(SaveLocations.SaveDirectory))
			{
				// New GNFS instance

				if (!Directory.Exists(SaveLocations.SaveDirectory))
				{
					Directory.CreateDirectory(SaveLocations.SaveDirectory);
					LogMessage($"Directory created: {SaveLocations.SaveDirectory}");
				}
				else
				{
					if (File.Exists(SaveLocations.SmoothRelations_SaveFile))
					{
						File.Delete(SaveLocations.SmoothRelations_SaveFile);
					}
					if (File.Exists(SaveLocations.RoughRelations_SaveFile))
					{
						File.Delete(SaveLocations.RoughRelations_SaveFile);
					}
					if (File.Exists(SaveLocations.QuadraticFactorPair_SaveFile))
					{
						File.Delete(SaveLocations.QuadraticFactorPair_SaveFile);
					}
					foreach (string freeRelationPath in SaveLocations.EnumerateFreeRelationFiles())
					{
						File.Delete(freeRelationPath);
					}
				}

				if (polyDegree == -1)
				{
					this.PolynomialDegree = CalculateDegree(n);
				}
				else
				{
					this.PolynomialDegree = polyDegree;
				}
				this.PolynomialBase = polynomialBase;

				if (cancelToken.IsCancellationRequested) { return; }

				ConstructNewPolynomial(this.PolynomialBase, this.PolynomialDegree);
				LogMessage($"Polynomial constructed: {this.CurrentPolynomial}");
				LogMessage($"Polynomial base: {this.PolynomialBase}");

				if (cancelToken.IsCancellationRequested) { return; }

				CaclulatePrimeFactorBaseBounds(primeBound);

				if (cancelToken.IsCancellationRequested) { return; }

				SetPrimeFactorBases();

				if (cancelToken.IsCancellationRequested) { return; }

				NewFactorPairCollections(cancelToken);
				LogMessage($"Factor bases populated.");

				if (cancelToken.IsCancellationRequested) { return; }

				CurrentRelationsProgress = new PolyRelationsSieveProgress(this, relationQuantity, relationValueRange);
				LogMessage($"Relations container initialized. Target quantity: {relationQuantity}");

				Serialization.Save.All(this);
			}
		}

		#endregion

		public bool IsFactor(BigInteger toCheck)
		{
			return ((N % toCheck) == 0);
		}


		#region New Factorization

		// Values were obtained from the paper:
		// "Polynomial Selection for the Number Field Sieve Integer factorization Algorithm" - by Brian Antony Murphy
		// Table 3.1, page 44
		private static int CalculateDegree(BigInteger n)
		{
			int result = 2;
			int base10 = n.ToString().Length;

			if (base10 < 65)
			{
				result = 3;
			}
			else if (base10 >= 65 && base10 < 125)
			{
				result = 4;
			}
			else if (base10 >= 125 && base10 < 225)
			{
				result = 5;
			}
			else if (base10 >= 225 && base10 < 315)
			{
				result = 6;
			}
			else if (base10 >= 315)
			{
				result = 7;
			}

			return result;
		}

		private void GetPrimeBoundsApproximation()
		{
			BigInteger bound = new BigInteger();

			int base10 = N.ToString().Length; //N.NthRoot(10, ref remainder);
			if (base10 <= 10)
			{
				bound = 100;//(int)((int)N.NthRoot(_degree, ref remainder) * 1.5); // 60;
			}
			else if (base10 <= 18)
			{
				bound = base10 * 1000;//(int)((int)N.NthRoot(_degree, ref remainder) * 1.5); // 60;
			}
			else if (base10 <= 100)
			{
				bound = 100000;
			}
			else if (base10 > 100 && base10 <= 150)
			{
				bound = 250000;
			}
			else if (base10 > 150 && base10 <= 200)
			{
				bound = 125000000;
			}
			else if (base10 > 200)
			{
				bound = 250000000;
			}

			SetPrimeFactorBases();
		}

		public void CaclulatePrimeFactorBaseBounds(BigInteger bound)
		{
			PrimeFactorBase = new FactorBase();

			PrimeFactorBase.RationalFactorBaseMax = bound;
			PrimeFactorBase.AlgebraicFactorBaseMax = (PrimeFactorBase.RationalFactorBaseMax) * 3;

			PrimeFactorBase.QuadraticBaseCount = CalculateQuadraticBaseSize(PolynomialDegree);

			PrimeFactorBase.QuadraticFactorBaseMin = PrimeFactorBase.AlgebraicFactorBaseMax + 20;
			PrimeFactorBase.QuadraticFactorBaseMax = PrimeFactory.GetApproximateValueFromIndex((UInt64)(PrimeFactorBase.QuadraticFactorBaseMin + PrimeFactorBase.QuadraticBaseCount));

			LogMessage($"Rational  Factor Base Bounds: Min: - Max: {PrimeFactorBase.RationalFactorBaseMax}");
			LogMessage($"Algebraic Factor Base Bounds: Min: - Max: {PrimeFactorBase.AlgebraicFactorBaseMax}");
			LogMessage($"Quadratic Factor Base Bounds: Min: {PrimeFactorBase.QuadraticFactorBaseMin} Max: {PrimeFactorBase.QuadraticFactorBaseMax}");

			Serialization.Save.All(this);
			LogMessage("Saved prime factor base bounds.");
		}

		public void SetPrimeFactorBases()
		{
			LogMessage($"Constructing new prime bases (- of 3)...");

			PrimeFactory.IncreaseMaxValue(PrimeFactorBase.QuadraticFactorBaseMax);

			PrimeFactorBase.RationalFactorBase = PrimeFactory.GetPrimesTo(PrimeFactorBase.RationalFactorBaseMax);
			//Serialization.Save.FactorBase.Rational(this);
			LogMessage($"Completed rational prime base (1 of 3).");

			PrimeFactorBase.AlgebraicFactorBase = PrimeFactory.GetPrimesTo(PrimeFactorBase.AlgebraicFactorBaseMax);
			//Serialization.Save.FactorBase.Algebraic(this);
			LogMessage($"Completed algebraic prime base (2 of 3).");

			PrimeFactorBase.QuadraticFactorBase = PrimeFactory.GetPrimesFrom(PrimeFactorBase.QuadraticFactorBaseMin).Take(PrimeFactorBase.QuadraticBaseCount);
			//Serialization.Save.FactorBase.Quadratic(this);
			LogMessage($"Completed quadratic prime base (3 of 3).");
		}

		private static int CalculateQuadraticBaseSize(int polyDegree)
		{
			int result = -1;

			if (polyDegree <= 3)
			{
				result = 10;
			}
			else if (polyDegree == 4)
			{
				result = 20;
			}
			else if (polyDegree == 5 || polyDegree == 6)
			{
				result = 40;
			}
			else if (polyDegree == 7)
			{
				result = 80;
			}
			else if (polyDegree >= 8)
			{
				result = 100;
			}

			return result;
		}

		private void ConstructNewPolynomial(BigInteger polynomialBase, int polyDegree)
		{
			CurrentPolynomial = new Polynomial(N, polynomialBase, polyDegree);

			/* Turns out, this may actually make the absolute value of the relation norms larger, *
             * which is no good because you are hoping many of them are going to be smooth.       */
			//Polynomial.MakeCoefficientsSmaller(CurrentPolynomial, polynomialBase);

			PolynomialCollection.Add(CurrentPolynomial);
			Serialization.Save.All(this);
		}

		private void NewFactorPairCollections(CancellationToken cancelToken)
		{
			LogMessage($"Constructing new factor bases (- of 3)...");

			if (!RationalFactorPairCollection.Any())
			{
				RationalFactorPairCollection = FactorPairCollection.Factory.BuildRationalFactorPairCollection(this);
			}
			Serialization.Save.FactorPair.Rational(this);
			LogMessage($"Completed rational factor base (1 of 3).");


			if (cancelToken.IsCancellationRequested) { return; }
			if (!AlgebraicFactorPairCollection.Any())
			{
				AlgebraicFactorPairCollection = FactorPairCollection.Factory.BuildAlgebraicFactorPairCollection(cancelToken, this);
			}
			Serialization.Save.FactorPair.Algebraic(this);
			LogMessage($"Completed algebraic factor base (2 of 3).");


			if (cancelToken.IsCancellationRequested) { return; }
			if (!QuadraticFactorPairCollection.Any())
			{
				QuadraticFactorPairCollection = FactorPairCollection.Factory.BuildQuadraticFactorPairCollection(cancelToken, this);
			}
			Serialization.Save.FactorPair.Quadratic(this);
			LogMessage($"Completed quadratic factor base (3 of 3).");

			if (cancelToken.IsCancellationRequested) { return; }
		}

		#endregion

		public static List<Relation[]> GroupRoughNumbers(List<Relation> roughNumbers)
		{
			IEnumerable<Relation> input1 = roughNumbers.OrderBy(rp => rp.AlgebraicQuotient).ThenBy(rp => rp.RationalQuotient);
			//IEnumerable<Relation> input2 = roughNumbers.OrderBy(rp => rp.RationalQuotient).ThenBy(rp => rp.AlgebraicQuotient);

			Relation lastItem = null;
			List<Relation[]> results = new List<Relation[]>();
			foreach (Relation pair in input1)
			{
				if (lastItem == null)
				{
					lastItem = pair;
				}
				else if (pair.AlgebraicQuotient == lastItem.AlgebraicQuotient && pair.RationalQuotient == lastItem.RationalQuotient)
				{
					results.Add(new Relation[] { pair, lastItem });
					lastItem = null;
				}
				else
				{
					lastItem = pair;
				}
			}

			return results;
		}

		public static List<Relation> MultiplyLikeRoughNumbers(GNFS gnfs, List<Relation[]> likeRoughNumbersGroups)
		{
			List<Relation> result = new List<Relation>();

			foreach (Relation[] likePair in likeRoughNumbersGroups)
			{
				var As = likePair.Select(lp => lp.A).ToList();
				var Bs = likePair.Select(lp => lp.B).ToList();

				int a = (int)(As[0] + Bs[0]) * (int)(As[0] - Bs[0]);//(int)Math.Round(Math.Sqrt(As.Sum()));
				uint b = (uint)(As[1] + Bs[1]) * (uint)(As[1] - Bs[1]);//(int)Math.Round(Math.Sqrt(Bs.Sum()));

				if (a > 0 && b > 0)
				{
					result.Add(new Relation(gnfs, a, b));
				}
			}

			return result;
		}

		public void LogMessage(string message = "")
		{
			if (LogFunction != null)
			{
				LogFunction.Invoke("　" + message);
			}
		}

		public bool SetFactorizationSolution(BigInteger p, BigInteger q)
		{
			BigInteger n = p * q;

			if (n == this.N)
			{
				Factorization = new Solution(p, q);
				string path = Path.Combine(SaveLocations.SaveDirectory, "Solution.txt");
				File.WriteAllText(path, Factorization.ToString());
				return true;
			}
			return false;
		}

		#region ToString

		public override string ToString()
		{
			StringBuilder result = new StringBuilder();

			result.AppendLine($"N = {N}");
			result.AppendLine();
			result.AppendLine($"Polynomial(degree: {PolynomialDegree}, base: {PolynomialBase}):");
			result.AppendLine("ƒ(m) = " + CurrentPolynomial.ToString());
			result.AppendLine();
			result.AppendLine("Prime Factor Base Bounds:");
			result.AppendLine($"RationalFactorBase : {PrimeFactorBase.RationalFactorBaseMax}");
			result.AppendLine($"AlgebraicFactorBase: {PrimeFactorBase.AlgebraicFactorBaseMax}");
			result.AppendLine($"QuadraticPrimeBase Range: {PrimeFactorBase.QuadraticFactorBaseMin} - {PrimeFactorBase.QuadraticFactorBaseMax}");
			result.AppendLine($"QuadraticPrimeBase Count: {PrimeFactorBase.QuadraticBaseCount}");
			result.AppendLine();
			result.AppendLine($"RFB - Rational Factor Base - Count: {RationalFactorPairCollection.Count} - Array of (p, m % p) with prime p");
			result.AppendLine(RationalFactorPairCollection.ToString(200));
			result.AppendLine();
			result.AppendLine($"AFB - Algebraic Factor Base - Count: {AlgebraicFactorPairCollection.Count} - Array of (p, r) such that ƒ(r) ≡ 0 (mod p) and p is prime");
			result.AppendLine(AlgebraicFactorPairCollection.ToString(200));
			result.AppendLine();
			result.AppendLine($"QFB - Quadratic Factor Base - Count: {QuadraticFactorPairCollection.Count} - Array of (p, r) such that ƒ(r) ≡ 0 (mod p) and p is prime");
			result.AppendLine(QuadraticFactorPairCollection.ToString());
			result.AppendLine();

			result.AppendLine();
			result.AppendLine();

			return result.ToString();
		}

		#endregion
	}
}

using System;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore
{
    public static class SieveRange
    {
        public static IEnumerable<BigInteger> GetSieveRange(BigInteger maximumRange)
        {
            return GetSieveRangeContinuation(1, maximumRange);
        }

        public static IEnumerable<BigInteger> GetSieveRangeContinuation(BigInteger currentValue, BigInteger maximumRange)
        {
            BigInteger max = maximumRange;
            BigInteger counter = BigInteger.Abs(currentValue);
            bool flipFlop = !(currentValue.Sign == -1);

            while (counter <= max)
            {
                if (flipFlop)
                {
                    yield return counter;
                    flipFlop = false;
                }
                else if (!flipFlop)
                {
                    yield return -counter;
                    counter++;
                    flipFlop = true;
                }
            }
        }
    }
}

using System;
using System.Linq;
using System.Text;
using System.Numerics;
using System.Threading.Tasks;
using System.Collections.Generic;
using System.Runtime.Serialization;
using System.IO;

namespace GNFSCore
{
	public class Solution
	{
		[DataMember]
		public BigInteger P { get; private set; }
		[DataMember]
		public BigInteger Q { get; private set; }

		public Solution(BigInteger p, BigInteger q)
		{
			P = p;
			Q = q;
		}

		public override string ToString()
		{
			StringBuilder sb = new StringBuilder();
			sb.AppendLine($"N = {(P * Q)}");
			sb.AppendLine();
			sb.AppendLine($"P = {BigInteger.Max(P, Q)}");
			sb.AppendLine($"Q = {BigInteger.Min(P, Q)}");
			sb.AppendLine();

			return sb.ToString();
		}
	}
}

using System;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore
{
	public static class StaticRandom
	{
		private static readonly Random rand = new Random();
		static StaticRandom()
		{
			int counter = rand.Next(100, 200);
			while (counter-- > 0)
			{
				rand.Next();
			}
		}

		public static int Next()
		{
			return rand.Next();
		}

		public static int Next(int maxValue)
		{
			return rand.Next(maxValue);
		}

		public static int Next(int minValue, int maxValue)
		{
			return rand.Next(minValue, maxValue);
		}

		public static double NextDouble()
		{
			return rand.NextDouble();
		}

		public static void NextBytes(byte[] bytes)
		{
			rand.NextBytes(bytes);
		}

		/// <summary>
		///  Picks a random number from a range, where such numbers will be chosen uniformly across the entire range.
		/// </summary>
		public static BigInteger NextBigInteger(BigInteger lower, BigInteger upper)
		{
			if (lower > upper) { throw new ArgumentOutOfRangeException("Upper must be greater than upper"); }

			BigInteger delta = upper - lower;
			byte[] deltaBytes = delta.ToByteArray();
			byte[] buffer = new byte[deltaBytes.Length];
			deltaBytes = null;

			BigInteger result;
			while (true)
			{
				NextBytes(buffer);

				result = new BigInteger(buffer) + lower;

				if (result >= lower && result <= upper)
				{
					buffer = null;
					return result;
				}
			}
		}
	}
}


