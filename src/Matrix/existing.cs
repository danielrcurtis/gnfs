using System;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore.Matrix
{
	using IntegerMath;

	public class GaussianMatrix
	{
		public List<bool[]> Matrix { get { return M; } }
		public bool[] FreeVariables { get { return freeCols; } }

		public int RowCount { get { return M.Count; } }
		public int ColumnCount { get { return M.Any() ? M.First().Length : 0; } }

		private List<bool[]> M;
		private bool[] freeCols;
		private bool eliminationStep;

		private GNFS _gnfs;
		private List<Relation> relations;
		public Dictionary<int, Relation> ColumnIndexRelationDictionary;
		private List<Tuple<Relation, bool[]>> relationMatrixTuple;

		public GaussianMatrix(GNFS gnfs, List<Relation> rels)
		{
			_gnfs = gnfs;
			relationMatrixTuple = new List<Tuple<Relation, bool[]>>();
			eliminationStep = false;
			freeCols = new bool[0];
			M = new List<bool[]>();

			relations = rels;

			List<GaussianRow> relationsAsRows = new List<GaussianRow>();

			foreach (Relation rel in relations)
			{
				GaussianRow row = new GaussianRow(_gnfs, rel);

				relationsAsRows.Add(row);
			}

			//List<GaussianRow> orderedRows = relationsAsRows.OrderBy(row1 => row1.LastIndexOfAlgebraic).ThenBy(row2 => row2.LastIndexOfQuadratic).ToList();

			List<GaussianRow> selectedRows = relationsAsRows.Take(_gnfs.CurrentRelationsProgress.SmoothRelationsRequiredForMatrixStep).ToList();

			int maxIndexRat = selectedRows.Select(row => row.LastIndexOfRational).Max();
			int maxIndexAlg = selectedRows.Select(row => row.LastIndexOfAlgebraic).Max();
			int maxIndexQua = selectedRows.Select(row => row.LastIndexOfQuadratic).Max();

			foreach (GaussianRow row in selectedRows)
			{
				row.ResizeRationalPart(maxIndexRat);
				row.ResizeAlgebraicPart(maxIndexAlg);
				row.ResizeQuadraticPart(maxIndexQua);
			}

			GaussianRow exampleRow = selectedRows.First();
			int newLength = exampleRow.GetBoolArray().Length;

			newLength++;

			selectedRows = selectedRows.Take(newLength).ToList();


			foreach (GaussianRow row in selectedRows)
			{
				relationMatrixTuple.Add(new Tuple<Relation, bool[]>(row.SourceRelation, row.GetBoolArray()));
			}
		}

		public void TransposeAppend()
		{
			List<bool[]> result = new List<bool[]>();
			ColumnIndexRelationDictionary = new Dictionary<int, Relation>();

			int index = 0;
			int numRows = relationMatrixTuple[0].Item2.Length;
			while (index < numRows)
			{
				ColumnIndexRelationDictionary.Add(index, relationMatrixTuple[index].Item1);

				List<bool> newRow = relationMatrixTuple.Select(bv => bv.Item2[index]).ToList();
				newRow.Add(false);
				result.Add(newRow.ToArray());

				index++;
			}

			M = result;
			freeCols = new bool[M.Count];
		}

		public void Elimination()
		{
			if (eliminationStep)
			{
				return;
			}

			int numRows = RowCount;
			int numCols = ColumnCount;

			freeCols = Enumerable.Repeat(false, numCols).ToArray();

			int h = 0;

			for (int i = 0; i < numRows && h < numCols; i++)
			{
				bool next = false;

				if (M[i][h] == false)
				{
					int t = i + 1;

					while (t < numRows && M[t][h] == false)
					{
						t++;
					}

					if (t < numRows)
					{
						//swap rows M[i] and M[t]

						bool[] temp = M[i];
						M[i] = M[t];
						M[t] = temp;
						temp = null;
					}
					else
					{
						freeCols[h] = true;
						i--;
						next = true;
					}
				}
				if (next == false)
				{
					for (int j = i + 1; j < numRows; j++)
					{
						if (M[j][h] == true)
						{
							// Add rows
							// M [j] ← M [j] + M [i]

							M[j] = Add(M[j], M[i]);
						}
					}
					for (int j = 0; j < i; j++)
					{
						if (M[j][h] == true)
						{
							// Add rows
							// M [j] ← M [j] + M [i]

							M[j] = Add(M[j], M[i]);
						}
					}
				}
				h++;
			}

			eliminationStep = true;
		}

		public List<Relation> GetSolutionSet(int numberOfSolutions)
		{
			bool[] solutionSet = GetSolutionFlags(numberOfSolutions);

			int index = 0;
			int max = ColumnIndexRelationDictionary.Count;

			List<Relation> result = new List<Relation>();
			while (index < max)
			{
				if (solutionSet[index] == true)
				{
					result.Add(ColumnIndexRelationDictionary[index]);
				}

				index++;
			}

			return result;
		}

		private bool[] GetSolutionFlags(int numSolutions)
		{
			if (!eliminationStep)
			{
				throw new Exception("Must call Elimination() method first!");
			}

			if (numSolutions < 1)
			{
				throw new ArgumentException($"{nameof(numSolutions)} must be greater than 1.");
			}

			int numRows = RowCount;
			int numCols = ColumnCount;

			if (numSolutions >= numCols)
			{
				throw new ArgumentException($"{nameof(numSolutions)} must be less than the column count.");
			}

			bool[] result = new bool[numCols];

			int j = -1;
			int i = numSolutions;

			while (i > 0)
			{
				j++;

				while (freeCols[j] == false)
				{
					j++;
				}

				i--;
			}

			result[j] = true;

			for (i = 0; i < numRows - 1; i++)
			{
				if (M[i][j] == true)
				{
					int h = i;
					while (h < j)
					{
						if (M[i][h] == true)
						{
							result[h] = true;
							break;
						}
						h++;
					}
				}
			}

			return result;
		}

		public static bool[] Add(bool[] left, bool[] right)
		{
			if (left.Length != right.Length) throw new ArgumentException($"Both vectors must have the same length.");

			int length = left.Length;
			bool[] result = new bool[length];

			int index = 0;
			while (index < length)
			{
				result[index] = left[index] ^ right[index];
				index++;
			}

			return result;
		}

		public static string VectorToString(bool[] vector)
		{
			return string.Join(",", vector.Select(b => b ? '1' : '0'));
		}

		public static string MatrixToString(List<bool[]> matrix)
		{
			return string.Join(Environment.NewLine, matrix.Select(i => VectorToString(i)));
		}

		public override string ToString()
		{
			return MatrixToString(M);
		}
	}
}

using System;
using System.Linq;
using System.Numerics;
using System.Collections.Generic;

namespace GNFSCore.Matrix
{
	using Factors;
	using IntegerMath;

	public class GaussianRow
	{
		public bool Sign { get; set; }

		public List<bool> RationalPart { get; set; }
		public List<bool> AlgebraicPart { get; set; }
		public List<bool> QuadraticPart { get; set; }

		public int LastIndexOfRational { get { return RationalPart.LastIndexOf(true); } }
		public int LastIndexOfAlgebraic { get { return AlgebraicPart.LastIndexOf(true); } }
		public int LastIndexOfQuadratic { get { return QuadraticPart.LastIndexOf(true); } }

		public Relation SourceRelation { get; private set; }

		public GaussianRow(GNFS gnfs, Relation relation)
		{
			SourceRelation = relation;

			if (relation.RationalNorm.Sign == -1)
			{
				Sign = true;
			}
			else
			{
				Sign = false;
			}

			FactorPairCollection qfb = gnfs.QuadraticFactorPairCollection;

			BigInteger rationalMaxValue = gnfs.PrimeFactorBase.RationalFactorBaseMax;
			BigInteger algebraicMaxValue = gnfs.PrimeFactorBase.AlgebraicFactorBaseMax;

			RationalPart = GetVector(relation.RationalFactorization, rationalMaxValue).ToList();
			AlgebraicPart = GetVector(relation.AlgebraicFactorization, algebraicMaxValue).ToList();
			QuadraticPart = qfb.Select(qf => QuadraticResidue.GetQuadraticCharacter(relation, qf)).ToList();
		}

		protected static bool[] GetVector(CountDictionary primeFactorizationDict, BigInteger maxValue)
		{
			int primeIndex = PrimeFactory.GetIndexFromValue(maxValue);

			bool[] result = new bool[primeIndex];

			if (primeFactorizationDict.Any())
			{
				foreach (KeyValuePair<BigInteger, BigInteger> kvp in primeFactorizationDict)
				{
					if (kvp.Key > maxValue)
					{
						continue;
					}
					if (kvp.Key == -1)
					{
						continue;
					}
					if (kvp.Value % 2 == 0)
					{
						continue;
					}

					int index = PrimeFactory.GetIndexFromValue(kvp.Key);
					result[index] = true;
				}
			}

			return result;
		}
				
		public bool[] GetBoolArray()
		{
			List<bool> result = new List<bool>() { Sign };
			result.AddRange(RationalPart);
			result.AddRange(AlgebraicPart);
			result.AddRange(QuadraticPart);
			//result.Add(false);
			return result.ToArray();
		}

		public void ResizeRationalPart(int size)
		{
			RationalPart = RationalPart.Take(size + 1).ToList();
		}

		public void ResizeAlgebraicPart(int size)
		{
			AlgebraicPart = AlgebraicPart.Take(size + 1).ToList();
		}

		public void ResizeQuadraticPart(int size)
		{
			QuadraticPart = QuadraticPart.Take(size + 1).ToList();
		}
	}
}

using System;
using System.Linq;
using System.Numerics;
using System.Threading;
using System.Collections.Generic;

namespace GNFSCore.Matrix
{
	using IntegerMath;

	public static class MatrixSolve
	{
		public static void GaussianSolve(CancellationToken cancelToken, GNFS gnfs)
		{
			Serialization.Save.Relations.Smooth.Append(gnfs); // Persist any relations not already persisted to disk

			// Because some operations clear this collection after persisting unsaved relations (to keep memory usage light)...
			// We completely reload the entire relations collection from disk.
			// This ensure that all the smooth relations are available for the matrix solving step.
			Serialization.Load.Relations.Smooth(ref gnfs);


			List<Relation> smoothRelations = gnfs.CurrentRelationsProgress.SmoothRelations.ToList();

			int smoothCount = smoothRelations.Count;

			BigInteger requiredRelationsCount = gnfs.CurrentRelationsProgress.SmoothRelationsRequiredForMatrixStep;

			GNFS.LogFunction($"Total relations count: {smoothCount}");
			GNFS.LogFunction($"Relations required to proceed: {requiredRelationsCount}");

			while (smoothRelations.Count >= requiredRelationsCount)
			{
				// Randomly select n relations from smoothRelations
				List<Relation> selectedRelations = new List<Relation>();
				while (
						selectedRelations.Count < requiredRelationsCount
						||
						selectedRelations.Count % 2 != 0 // Force number of relations to be even
					)
				{
					int randomIndex = StaticRandom.Next(0, smoothRelations.Count);
					selectedRelations.Add(smoothRelations[randomIndex]);
					smoothRelations.RemoveAt(randomIndex);
				}

				GaussianMatrix gaussianReduction = new GaussianMatrix(gnfs, selectedRelations);
				gaussianReduction.TransposeAppend();
				gaussianReduction.Elimination();

				int number = 1;
				int solutionCount = gaussianReduction.FreeVariables.Count(b => b) - 1;
				List<List<Relation>> solution = new List<List<Relation>>();
				while (number <= solutionCount)
				{
					List<Relation> relations = gaussianReduction.GetSolutionSet(number);
					number++;

					BigInteger algebraic = relations.Select(rel => rel.AlgebraicNorm).Product();
					BigInteger rational = relations.Select(rel => rel.RationalNorm).Product();

					CountDictionary algCountDict = new CountDictionary();
					foreach (var rel in relations)
					{
						algCountDict.Combine(rel.AlgebraicFactorization);
					}

					bool isAlgebraicSquare = algebraic.IsSquare();
					bool isRationalSquare = rational.IsSquare();

					//gnfs.LogFunction("---");
					//gnfs.LogFunction($"Relations count: {relations.Count}");
					//gnfs.LogFunction($"(a,b) pairs: {string.Join(" ", relations.Select(rel => $"({rel.A},{rel.B})"))}");
					//gnfs.LogFunction($"Rational  ∏(a+mb): IsSquare? {isRationalSquare} : {rational}");
					//gnfs.LogFunction($"Algebraic ∏ƒ(a/b): IsSquare? {isAlgebraicSquare} : {algebraic}");
					//gnfs.LogFunction($"Algebraic (factorization): {algCountDict.FormatStringAsFactorization()}");

					if (isAlgebraicSquare && isRationalSquare)
					{
						solution.Add(relations);
						gnfs.CurrentRelationsProgress.AddFreeRelationSolution(relations);
					}

					if (cancelToken.IsCancellationRequested)
					{
						break;
					}
				}

				if (cancelToken.IsCancellationRequested)
				{
					break;
				}
			}
		}
	}
}

