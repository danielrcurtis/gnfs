using System;
using System.Windows.Forms;

namespace GNFS_Winforms
{
	public class ControlBridge
	{
		private Control control;

		public ControlBridge(Control ctrl)
		{
			control = ctrl;
		}

		public void SetControlEnabledState(bool enabled)
		{
			SetControlEnabledState(control, enabled);
		}

		public void SetControlText(string text)
		{
			SetControlText(control, text);
		}

		public static void SetControlEnabledState(Control control, bool enabled)
		{
			if (control.IsDisposed || !control.IsHandleCreated)
			{
				throw new Exception();
			}

			if (control.InvokeRequired /* && !GNFSCore.DirectoryLocations.IsLinuxOS()*/)
			{
				control.Invoke(new Action(() => { SetControlEnabledState(control, enabled); }));
			}
			else
			{
				control.Enabled = enabled;
			}
		}

		public static void SetControlVisibleState(Control control, bool visible)
		{
			if (control.IsDisposed || !control.IsHandleCreated)
			{
				throw new Exception();
			}

			if (control.InvokeRequired /* && !GNFSCore.DirectoryLocations.IsLinuxOS()*/)
			{
				control.Invoke(new Action(() => { SetControlVisibleState(control, visible); }));
			}
			else
			{
				control.Visible = visible;
			}
		}

		public static void SetControlText(Control control, string text)
		{
			if (control.IsDisposed || !control.IsHandleCreated)
			{
				throw new Exception();
			}

			if (control.InvokeRequired /* && !GNFSCore.DirectoryLocations.IsLinuxOS()*/)
			{
				control.Invoke(new Action(() => { SetControlText(control, text); }));
			}
			else
			{
				control.Text = text;
			}
		}
	}
}

using System;
using System.IO;
using System.Linq;
using System.Numerics;
using System.Threading;

namespace GNFS_Winforms
{
	using GNFSCore;

	public partial class GnfsUiBridge
	{
		public static GNFS CreateGnfs(CancellationToken cancelToken, BigInteger n, BigInteger polyBase, int degree, BigInteger primeBound, int relationsTargetQuantity, int relationValueRange, bool createNewData = false)
		{
			GNFS gnfs = new GNFS(cancelToken, Logging.LogMessage, n, polyBase, degree, primeBound, relationsTargetQuantity, relationValueRange, createNewData);
			return gnfs;
		}

		public static GNFS LoadGnfs(BigInteger n)
		{
			string jsonFilename = Path.Combine(DirectoryLocations.GetSaveLocation(n), "GNFS.json");
			GNFS gnfs = Serialization.Load.All(jsonFilename);
			return gnfs;
		}
	}
}
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace GNFS_Winforms
{
	public static class TimeSpanExtensionMethods
	{
		public static string FormatString(this TimeSpan source)
		{
			bool subSecond = true;
			List<string> elapsedString = new List<string>();
			if (source.Days > 0)
			{
				elapsedString.Add($"{source.Days} Days");
				subSecond = false;
			}
			if (source.Hours > 0)
			{
				elapsedString.Add($"{source.Hours} Hours");
				subSecond = false;
			}
			if (source.Minutes > 0)
			{
				elapsedString.Add($"{source.Minutes} Minutes");
				subSecond = false;
			}
			if (source.Seconds > 0)
			{
				elapsedString.Add($"{source.Seconds}.{source.Milliseconds} Seconds");
				subSecond = false;
			}
			if (subSecond)
			{
				elapsedString.Add($"{source.Milliseconds} Milliseconds");
			}
			return string.Join(", ", elapsedString);
		}
	}
}

using System;
using System.Threading;

namespace GNFS_Winforms
{
	using GNFSCore;

	public partial class GnfsUiBridge
	{
		public static GNFS FindRelations(CancellationToken cancelToken, GNFS gnfs, bool oneRound)
		{
			while (!cancelToken.IsCancellationRequested)
			{
				if (gnfs.CurrentRelationsProgress.SmoothRelationsCounter >= gnfs.CurrentRelationsProgress.SmoothRelations_TargetQuantity)
				{
					gnfs.CurrentRelationsProgress.IncreaseTargetQuantity(100);
				}

				gnfs.CurrentRelationsProgress.GenerateRelations(cancelToken);

				Logging.LogMessage();
				Logging.LogMessage($"Sieving progress saved at:");
				Logging.LogMessage($"   A = {gnfs.CurrentRelationsProgress.A}");
				Logging.LogMessage($"   B = {gnfs.CurrentRelationsProgress.B}");
				Logging.LogMessage();

				if (oneRound)
				{
					break;
				}

				if (gnfs.CurrentRelationsProgress.SmoothRelationsCounter >= gnfs.CurrentRelationsProgress.SmoothRelations_TargetQuantity)
				{
					break;
				}
			}

			return gnfs;
		}
	}
}

using System;
using System.IO;
using System.Text;
using System.Linq;
using System.Numerics;
using System.Threading;
using System.Collections.Generic;
using ExtendedArithmetic;

namespace GNFS_Winforms
{
	using GNFSCore;
	using GNFSCore.SquareRoot;

	public partial class GnfsUiBridge
	{
		public static GNFS FindSquares(CancellationToken cancelToken, GNFS gnfs)
		{
			if (cancelToken.IsCancellationRequested)
			{
				return gnfs;
			}

			BigInteger polyBase = gnfs.PolynomialBase;
			List<List<Relation>> freeRelations = gnfs.CurrentRelationsProgress.FreeRelations;

			bool solutionFound = SquareFinder.Solve(cancelToken, gnfs);



			return gnfs;
		}
	}
}

using System;
using System.IO;
using System.Linq;
using System.Windows.Forms;

namespace GNFS_Winforms
{
	public static class Logging
	{
		public static MainForm PrimaryForm;
		public static TextBox OutputTextbox;
		public static bool FirstFindRelations = false;
		public static string OutputFilename = Path.GetFullPath(Settings.Log_FileName ?? DefaultLoggingFilename);
		public static string ExceptionLogFilename = Path.GetFullPath(DefaultExceptionLogFilename);


		private static int MaxLines = 200;
		private const string DefaultLoggingFilename = "Output.log.txt";
		private const string DefaultExceptionLogFilename = "Exceptions.log.txt";

		public static bool IsDebugMode()
		{
#if DEBUG
			return true;
#else
			return false;
#endif
		}

		public static void LogMessage()
		{
			LogMessage(string.Empty);
		}

		public static void LogMessage(string message, params object[] args)
		{
			LogMessage(args.Any() ? string.Format(message, args) : string.IsNullOrWhiteSpace(message) ? "(empty)" : message);
		}

		public static void LogMessage(string message)
		{
			string toLog = message + Environment.NewLine;
			CreateLogFileIfNotExists(OutputFilename);
			File.AppendAllText(OutputFilename, GetTimestamp() + toLog);
			LogTextbox(toLog);
		}

		public static void LogException(Exception ex, string message)
		{
			string toLog = (ex == null) ? Environment.NewLine + "Application encountered an error" : ex.ToString();

			if (!string.IsNullOrWhiteSpace(message))
				toLog += ": " + message;
			else
				toLog += "!";

			toLog += Environment.NewLine + Environment.NewLine;


			CreateLogFileIfNotExists(OutputFilename);
			File.AppendAllText(OutputFilename, GetTimestamp() + toLog);
			LogTextbox(toLog);
		}

		public static void LogTextbox(string message)
		{
			//if (GNFSCore.DirectoryLocations.IsLinuxOS())
			//{
			//	return;
			//}

			if (OutputTextbox.IsDisposed || !OutputTextbox.IsHandleCreated)
			{
				throw new Exception();
			}

			if (OutputTextbox.InvokeRequired /* && !GNFSCore.DirectoryLocations.IsLinuxOS()*/)
			{
				OutputTextbox.Invoke(new Action(() => { LogTextbox(message); }));
			}
			else
			{
				string toLog = message;
				if (PrimaryForm.IsWorking)
				{
					toLog = "\t" + message;
				}

				if (OutputTextbox.Lines.Length > MaxLines)
				{
					OutputTextbox.Clear();
				}
				OutputTextbox.AppendText(toLog);
			}
		}

		public static void CreateLogFileIfNotExists(string file)
		{
			string directory = Path.GetDirectoryName(file);
			if (!Directory.Exists(directory))
			{
				FirstFindRelations = true;
				Directory.CreateDirectory(directory);
			}
			if (!File.Exists(file))
			{
				string logHeader = $"Log created: {DateTime.Now}";
				string line = new string(Enumerable.Repeat('-', logHeader.Length).ToArray());

				File.WriteAllLines(file, new string[] { logHeader, line, Environment.NewLine });
			}
		}

		public static string GetTimestamp()
		{
			DateTime now = DateTime.Now;
			return $"[{now.DayOfYear}.{now.Year} @ {now.ToString("HH:mm:ss")}]  ";
		}
	}
}


using System;
using System.Linq;
using System.Numerics;
using System.Threading;
using System.Collections.Generic;

namespace GNFS_Winforms
{
	using GNFSCore;
	using GNFSCore.Matrix;

	public partial class GnfsUiBridge
	{
		public static GNFS MatrixSolveGaussian(CancellationToken cancelToken, GNFS gnfs)
		{
			MatrixSolve.GaussianSolve(cancelToken, gnfs);
			return gnfs;
		}
	}
}

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace GNFS_Winforms
{
	public static class Program
	{
		/// <summary>
		/// The main entry point for the application.
		/// </summary>
		[STAThread]
		static void Main()
		{
			Application.EnableVisualStyles();
			Application.SetCompatibleTextRenderingDefault(false);
			Application.ThreadException += Application_ThreadException;
			Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
			AppDomain.CurrentDomain.UnhandledException += CurrentDomain_UnhandledException;
			Application.Run(new MainForm());
		}

		private static void CurrentDomain_UnhandledException(object sender, UnhandledExceptionEventArgs e)
		{
			try
			{
				Logging.LogException((Exception)e.ExceptionObject, "CAUGHT A UNHANDLED _APPLICATION_ EXCEPTION");
			}
			catch
			{
			}
		}

		private static void Application_ThreadException(object sender, System.Threading.ThreadExceptionEventArgs e)
		{
			try
			{
				Logging.LogException(e.Exception, "ENCOUNTERED A UNTRAPPED _THREAD_ EXCEPTION");
			}
			catch
			{
			}
		}		
	}
}

using System;
using System.Collections.Generic;
using System.Linq;
using System.Web;

namespace GNFS_Winforms
{
    public static class Settings
    {
        public static string Log_FileName = SettingsReader.GetSettingValue<string>("Log.FileName");        

        public static string N = SettingsReader.GetSettingValue<string>("N");       
        public static string Degree = SettingsReader.GetSettingValue<string>("Degree");
        public static string Base = SettingsReader.GetSettingValue<string>("Base");
		public static string Bound = SettingsReader.GetSettingValue<string>("Bound");       
        public static string RelationQuantity = SettingsReader.GetSettingValue<string>("RelationQuantity");
        public static string RelationValueRange = SettingsReader.GetSettingValue<string>("RelationValueRange");
	}
}

using System;
using System.Linq;
using System.Configuration;
using System.Collections.Generic;

namespace GNFS_Winforms
{
	public static class SettingsReader
	{
		public static T GetSettingValue<T>(string SettingName)
		{
			try
			{
				if (SettingExists(SettingName))
				{
					T result = (T)Convert.ChangeType(ConfigurationManager.AppSettings[SettingName], typeof(T));
					if (result != null)
					{
						return result;
					}
				}
			}
			catch (Exception ex)
			{
				Logging.LogException(ex, $"{nameof(SettingsReader)}.{nameof(GetSettingValue)} threw an exception.");
			}

			return default(T);
		}

		public static bool SettingExists(string SettingName)
		{
			try
			{
				if (string.IsNullOrWhiteSpace(SettingName))
				{
					return false;
				}
				else if (!ConfigurationManager.AppSettings.AllKeys.Contains(SettingName))
				{
					return false;
				}
				else if (string.IsNullOrWhiteSpace(ConfigurationManager.AppSettings[SettingName]))
				{
					return false;
				}

				return true;
			}
			catch (Exception ex)
			{
				Logging.LogException(ex, $"{nameof(SettingsReader)}.{nameof(SettingExists)} threw an exception.");
				return false;
			}
		}
	}
}
