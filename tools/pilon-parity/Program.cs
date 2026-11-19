// Runs PilonHydraulika's WaterAnalyzer on each network <name>.json in the
// folder given (crates/creabim-services/tests/parity) and writes what it
// finds to <name>.expected.json.
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using PilonHydraulika.Core;

static class Parity
{
	static readonly JsonSerializerOptions Opts = new JsonSerializerOptions
	{
		WriteIndented = true,
		Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
		Converters = { new JsonStringEnumConverter() },
	};

	static T En<T>(JsonNode? v) where T : struct, Enum =>
		v == null ? default : Enum.Parse<T>(v.GetValue<string>().Replace("_", ""), ignoreCase: true);

	static Vec3 V(JsonNode? a) => new Vec3((double)a![0]!, (double)a[1]!, (double)a[2]!);

	static PipeSize Size(JsonNode a) => new PipeSize { NominalMm = (double)a[0]!, InnerMm = (double)a[1]! };

	static JsonArray? Out(PipeSize? s) => s == null ? null : new JsonArray(s.NominalMm, s.InnerMm);

	static int Main(string[] args)
	{
		string dir = args.Length > 0 ? args[0] : "crates/creabim-services/tests/parity";
		int n = 0;
		foreach (string path in Directory.GetFiles(dir, "*.json").Where(p => !p.EndsWith(".expected.json")).OrderBy(p => p))
		{
			JsonNode c = JsonNode.Parse(File.ReadAllText(path))!;
			Settings s = JsonSerializer.Deserialize<Settings>((c["settings"] ?? new JsonObject()).ToJsonString(), Opts) ?? new Settings();
			var nodes = new List<Node>();
			foreach (JsonNode j in c["nodes"]!.AsArray()!)
			{
				var x = new Node
				{
					Id = (string)j["id"]!,
					Name = (string?)j["fixture"] ?? "",
					Kind = En<NodeKind>(j["kind"]),
					Fitting = En<FittingKind>(j["fitting"]),
					Role = En<AccessoryRole>(j["role"]),
					P1 = V(j["p1"]),
					P2 = V(j["p2"]),
					IsBranchFromParent = (bool?)j["branch"] ?? false,
					IsLinkedEquipment = (bool?)j["linked"] ?? false,
					LeafRequiredPa = (double?)j["leaf_required_pa"] ?? 0.0,
					ZetaOverride = (double?)j["zeta"],
					ElbowAngleDeg = (double?)j["angle"] ?? 90.0,
				};
				x.LengthM = (x.P2 - x.P1).Length;
				if (x.Kind == NodeKind.Pipe)
				{
					x.NominalMm = (double)j["nominal"]!;
					x.InnerDiameterM = (double)j["inner"]! / 1000.0;
					x.RoughnessM = (double)j["roughness"]!;
					x.MaterialName = (string)j["material"]!;
					x.AvailableSizes = j["sizes"]!.AsArray().Select(a => Size(a!)).ToList();
					if (j["manual"] != null) x.ManualSize = Size(j["manual"]!);
				}
				if (j["fixture"] != null)
				{
					x.Fixture = s.MatchFixture((string)j["fixture"]!, "");
					x.Fixture.ConnectorNominalMm = (double?)j["connector_dn"] ?? 0.0;
				}
				int? parent = (int?)j["parent"];
				if (parent.HasValue) nodes[parent.Value].AddChild(x);
				nodes.Add(x);
			}
			SystemKind kind = (string)c["kind"]! == "hot" ? SystemKind.HotWater : SystemKind.ColdWater;
			NetworkResult r = new WaterAnalyzer(s).Run(nodes[0], kind, (string)c["name"]!, (bool)c["root_is_connection"]!, (double?)c["available_kpa"]);
			var outNodes = new JsonArray();
			foreach (Node x in nodes)
			{
				outNodes.Add(new JsonObject
				{
					["id"] = x.Id,
					["flow_ls"] = x.FlowLs,
					["sum_lu"] = x.SumLU,
					["min_nominal_mm"] = x.MinNominalMm,
					["proposed"] = Out(x.ProposedSize),
					["velocity_ms"] = x.VelocityMs,
					["r_pa_per_m"] = x.RPaPerM,
					["linear_loss_pa"] = x.LinearLossPa,
					["local_loss_pa"] = x.LocalLossPa,
					["fixed_loss_pa"] = x.FixedLossPa,
					["zeta"] = x.Zeta,
					["on_critical_path"] = x.OnCriticalPath,
				});
			}
			var o = new JsonObject
			{
				["root_flow_ls"] = r.RootFlowLs,
				["required_pa"] = r.RequiredPa,
				["required_pa_current"] = r.RequiredPaCurrent,
				["available_pa"] = r.AvailablePa,
				["critical_leaf"] = r.CriticalLeaf?.Id,
				["critical_path"] = new JsonArray(r.CriticalPath.Select(x => (JsonNode)JsonValue.Create(x.Id)!).ToArray()),
				["nodes"] = outNodes,
			};
			File.WriteAllText(path[..^".json".Length] + ".expected.json", o.ToJsonString(Opts) + "\n");
			n++;
		}
		Console.WriteLine($"{n} networks written.");
		return 0;
	}
}
