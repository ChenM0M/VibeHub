using System;
using System.IO;
using System.Text;

class AgentArgvProbe
{
    public static void Main(string[] args)
    {
        var values = new string[args.Length - 1];
        Array.Copy(args, 1, values, 0, values.Length);
        // Publish atomically so the parent never observes a partial record.
        var temporary = args[0] + ".tmp";
        File.WriteAllText(temporary, string.Join("\0", values), new UTF8Encoding(false));
        File.Move(temporary, args[0]);
    }
}
