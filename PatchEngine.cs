using System;
using System.IO;
using System.Text;
using System.Linq;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.Reflection;
using System.Security.Cryptography;
using System.Web.Script.Serialization;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

static class PatchEngine {
    static readonly string[] Names = {"katamari.js", "manifest.json"};
    static readonly string[] Fixes = {"PiP com legendas", "Próximo episódio", "Avanço sem espera", "Controles discretos"};
    static readonly UTF8Encoding Utf8 = new UTF8Encoding(false, true);
    const string End = "'croptix-rust-fixes:end';\n";
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern SafeFileHandle CreateFile(string file,uint access,uint share,IntPtr security,uint mode,uint flags,IntPtr template);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern uint GetFinalPathNameByHandle(SafeFileHandle handle,StringBuilder path,uint size,uint flags);
    static string Canonical(string path) {
        using(var handle=CreateFile(path,0,7,IntPtr.Zero,3,0x02000000,IntPtr.Zero)) {
            if(handle.IsInvalid) throw new IOException("Não foi possível validar: "+path);
            var result=new StringBuilder(32768);
            uint length=GetFinalPathNameByHandle(handle,result,(uint)result.Capacity,0);
            if(length==0||length>=result.Capacity) throw new IOException("Caminho inválido: "+path);
            return result.ToString();
        }
    }
    static string Normal(string path) {
        if(path.StartsWith("\\\\?\\UNC\\",StringComparison.OrdinalIgnoreCase)) return "\\\\"+path.Substring(8);
        return path.StartsWith("\\\\?\\",StringComparison.Ordinal)?path.Substring(4):path;
    }
    static void NotRedirected(string path) {
        if(!string.Equals(Normal(Canonical(path)),Path.GetFullPath(path),StringComparison.OrdinalIgnoreCase))
            throw new IOException("Backup ou arquivo redirecionado por link: "+path);
    }
    static string Root(string folder) {
        if(!Directory.Exists(folder)) throw new IOException("A pasta informada não existe.");
        string root=Normal(Canonical(Path.GetFullPath(folder)));
        foreach(string name in Names) {
            string file=Path.Combine(root,name);
            if(!File.Exists(file)) throw new IOException("Arquivo esperado não encontrado: "+name+". Se houver dist, selecione essa pasta.");
            NotRedirected(file);
        }
        return root;
    }
    static FileStream Lock(string root) {
        string file=Path.Combine(root,".croptix-rust.lock");
        if(File.Exists(file)) NotRedirected(file);
        try { return new FileStream(file,FileMode.OpenOrCreate,FileAccess.Write,FileShare.None,4096,FileOptions.DeleteOnClose); }
        catch { throw new IOException("A pasta está em uso por outro patcher ou não permite gravação."); }
    }
    static string Resource(string name) {
        using(var stream=Assembly.GetExecutingAssembly().GetManifestResourceStream(name)) {
            if(stream==null) throw new IOException("Recurso ausente: "+name);
            using(var reader=new StreamReader(stream,Utf8,false)) return reader.ReadToEnd();
        }
    }
    static string Payload() { return "'croptix-rust-fixes:v2';\n"+Resource("runtime.js")+End; }
    static string Hash(byte[] bytes) { using(var sha=SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(bytes)).Replace("-","").ToLowerInvariant(); }
    static bool Equal(byte[] a,byte[] b) { return a.SequenceEqual(b); }
    static byte[][] Read(string root) { return Names.Select(n=>File.ReadAllBytes(Path.Combine(root,n))).ToArray(); }
    static string Stamp() { return ((DateTime.UtcNow.Ticks-621355968000000000L)*100m).ToString("0",CultureInfo.InvariantCulture); }
    static void Stage(string path,byte[] bytes) {
        using(var file=new FileStream(path,FileMode.CreateNew,FileAccess.Write,FileShare.None)) { file.Write(bytes,0,bytes.Length);file.Flush(true); }
    }
    static void Replace(string target,byte[] expected,byte[] bytes) {
        string temp=Path.Combine(Path.GetDirectoryName(target),".croptix-"+Guid.NewGuid().ToString("N")+".tmp");
        try {
            Stage(temp,bytes);NotRedirected(target);
            if(!Equal(File.ReadAllBytes(target),expected)) throw new IOException(Path.GetFileName(target)+" foi alterado por outro programa; arquivo preservado.");
            File.Replace(temp,target,null);
        } finally { if(File.Exists(temp)) File.Delete(temp); }
    }
    static void Transaction(string root,byte[][] before,byte[][] after) {
        var staged=new Dictionary<int,string>();var written=new List<int>();
        try {
            for(int i=0;i<Names.Length;i++) if(!Equal(before[i],after[i])) {
                string temp=Path.Combine(root,".croptix-"+Guid.NewGuid().ToString("N")+".tmp");staged.Add(i,temp);Stage(temp,after[i]);
            }
            foreach(var item in staged.OrderBy(x=>x.Key)) {
                string target=Path.Combine(root,Names[item.Key]);NotRedirected(target);
                if(!Equal(File.ReadAllBytes(target),before[item.Key])) throw new IOException(Names[item.Key]+" foi alterado por outro programa.");
                File.Replace(item.Value,target,null);written.Add(item.Key);
            }
        } catch {
            foreach(int i in written.AsEnumerable().Reverse()) Replace(Path.Combine(root,Names[i]),after[i],before[i]);
            throw;
        } finally { foreach(string temp in staged.Values) if(File.Exists(temp)) File.Delete(temp); }
    }
    static string Base(string main,string prefix) {
        if(main.StartsWith(prefix,StringComparison.Ordinal)) return main.Substring(prefix.Length);
        string previous="'croptix-rust-fixes:v1';\n"+Resource("runtime-v1.js")+End;
        if(main.StartsWith(previous,StringComparison.Ordinal)) return main.Substring(previous.Length);
        if(main.StartsWith("'croptix-rust-fixes:",StringComparison.Ordinal)) throw new IOException("O patch existente foi editado ou é de outra versão. Restaure o backup antes de aplicar.");
        string[] starts={"// <CROPTIX-INDEPENDENT-FIXES v1>","// <CROPTIX-PATCHER-PIP1>"};
        string[] ends={"// </CROPTIX-INDEPENDENT-FIXES>","// </CROPTIX-PATCHER-PIP1>"};
        for(int i=0;i<starts.Length;i++) if(main.StartsWith(starts[i],StringComparison.Ordinal)) {
            int at=main.IndexOf(ends[i],StringComparison.Ordinal);if(at<0) throw new IOException("Patch anterior incompleto.");
            if(i==0) {
                string head=main.Substring(0,at);int first=head.IndexOf('\n');int second=head.IndexOf('\n',first+1);
                if(first<0||second<0) throw new IOException("Patch anterior inválido.");
                string signature=head.Substring(first+1,second-first-1).Trim();
                if(!signature.StartsWith("// SHA256: ",StringComparison.Ordinal)||Hash(Utf8.GetBytes(head.Substring(second+1)))!=signature.Substring(11))
                    throw new IOException("O patch anterior foi editado. Nenhum arquivo alterado.");
            }
            string tail=main.Substring(at+ends[i].Length);
            if(tail.StartsWith("\r\n",StringComparison.Ordinal)) return tail.Substring(2);
            if(tail.StartsWith("\n",StringComparison.Ordinal)) return tail.Substring(1);
            throw new IOException("Patch anterior inválido.");
        }
        if(main.Contains("croptix.integrated-fixes.installed")||main.Contains("croptix.pip1.installed")) throw new IOException("Correções já incorporadas sem bloco reconhecido.");
        return main;
    }
    static object Get(object obj,string key) { var map=obj as Dictionary<string,object>;object value;return map!=null&&map.TryGetValue(key,out value)?value:null; }
    static object[] Array(object value) { return value as object[]; }
    static bool Separate(object value) { var name=value as string;return name!=null&&(name.Split('/','\\').Last()=="pip.js"||name.Split('/','\\').Last()=="pip1.js"); }
    static bool RemoveScripts(object entry,string key) {
        var map=entry as Dictionary<string,object>;var list=Array(Get(entry,key));if(map==null||list==null) return false;
        var filtered=list.Where(n=>!Separate(n)).ToArray();if(filtered.Length==list.Length) return false;
        map[key]=filtered;return true;
    }
    static JavaScriptSerializer Serializer() { return new JavaScriptSerializer {MaxJsonLength=int.MaxValue,RecursionLimit=256}; }
    static string Quote(string value) {
        var text=new StringBuilder("\"");foreach(char c in value) switch(c) {
            case '"':text.Append("\\\"");break;case '\\':text.Append("\\\\");break;
            case '\b':text.Append("\\b");break;case '\f':text.Append("\\f");break;case '\n':text.Append("\\n");break;
            case '\r':text.Append("\\r");break;case '\t':text.Append("\\t");break;
            default:if(c<32) text.Append("\\u"+((int)c).ToString("x4"));else text.Append(c);break;
        }return text.Append('"').ToString();
    }
    static string Json(object value,int depth) {
        string pad=new string(' ',depth*2),child=new string(' ',(depth+1)*2);
        var map=value as Dictionary<string,object>;
        if(map!=null) return map.Count==0?"{}":"{\n"+string.Join(",\n",map.OrderBy(x=>x.Key,StringComparer.Ordinal).Select(x=>child+Quote(x.Key)+": "+Json(x.Value,depth+1)))+"\n"+pad+"}";
        var array=value as object[];
        if(array!=null) return array.Length==0?"[]":"[\n"+string.Join(",\n",array.Select(x=>child+Json(x,depth+1)))+"\n"+pad+"]";
        if(value is string) return Quote((string)value);
        return Serializer().Serialize(value);
    }
    public static bool Apply(string folder,Action<string> log) {
        log("[PASTA] "+folder);string root=Root(folder);byte[][] before=Read(root);
        string main=Utf8.GetString(before[0]);object manifest;
        try {manifest=Serializer().DeserializeObject(Utf8.GetString(before[1]).TrimStart('\ufeff'));}
        catch {throw new IOException("manifest.json inválido. Nenhum arquivo alterado.");}
        var scripts=Array(Get(manifest,"content_scripts"));if(scripts==null) throw new IOException("Manifesto sem content_scripts.");
        if(!scripts.Any(s=>Equals(Get(s,"world"),"MAIN")&&Array(Get(s,"js"))!=null&&Array(Get(s,"js")).Any(n=>Equals(n,"katamari.js")))) throw new IOException("Katamari deve ser carregado no mundo MAIN.");
        if(!main.Contains("player-controls-root")||!main.Contains("nextEpisodeVM")) throw new IOException("Katamari incompatível com os controles esperados.");
        log("[OK] katamari.js e manifest.json validados.");string prefix=Payload(),source=Base(main,prefix);bool changed=false;
        foreach(var script in scripts) changed=RemoveScripts(script,"js")||changed;
        var resources=Array(Get(manifest,"web_accessible_resources"));if(resources!=null) foreach(var entry in resources) changed=RemoveScripts(entry,"resources")||changed;
        byte[][] after={Utf8.GetBytes(prefix+source),changed?Utf8.GetBytes(Json(manifest,0)+"\n"):before[1]};
        if(Equal(before[0],after[0])&&Equal(before[1],after[1])) {
            foreach(string fix in Fixes) log("[JÁ APLICADO] "+fix);log("[OK] Nenhum arquivo alterado. Nenhum novo backup criado.");return false;
        }
        using(Lock(root)) {
            byte[][] current=Read(root);for(int i=0;i<2;i++) if(!Equal(current[i],before[i])) throw new IOException("Arquivos alterados durante a validação.");
            string backups=Path.Combine(root,".croptix-rust-backups");if(Directory.Exists(backups)) NotRedirected(backups);else Directory.CreateDirectory(backups);NotRedirected(backups);
            string snapshot=Path.Combine(backups,"snapshot-"+Stamp());Directory.CreateDirectory(snapshot);
            for(int i=0;i<2;i++) Stage(Path.Combine(snapshot,Names[i]),before[i]);
            var info=new Dictionary<string,object>{{"folder",Canonical(root)},{"created",Stamp()},{"before",before.Select(Hash).ToArray()},{"after",after.Select(Hash).ToArray()}};
            Stage(Path.Combine(snapshot,"info.json"),Utf8.GetBytes(Serializer().Serialize(info)));log("[BACKUP] "+snapshot);
            Transaction(root,before,after);
        }
        for(int i=0;i<2;i++) log("["+(Equal(before[i],after[i])?"SEM ALTERAÇÃO":"ALTERADO")+"] "+Names[i]);
        foreach(string fix in Fixes) log("["+(main.StartsWith(prefix,StringComparison.Ordinal)?"JÁ APLICADO":"APLICADO")+"] "+fix);
        return true;
    }
    public static void Restore(string folder,Action<string> log) {
        log("[PASTA] "+folder);string root=Root(folder);
        using(Lock(root)) {
            string backups=Path.Combine(root,".croptix-rust-backups");if(!Directory.Exists(backups)) throw new IOException("Nenhum backup deste patcher encontrado.");NotRedirected(backups);
            byte[][] before=Read(root);
            foreach(string snapshot in Directory.GetDirectories(backups,"snapshot-*").OrderByDescending(s=>s,StringComparer.Ordinal)) {
                byte[][] after;
                try {
                    NotRedirected(snapshot);foreach(string name in Names.Concat(new[]{"info.json"})) NotRedirected(Path.Combine(snapshot,name));
                    object info=Serializer().DeserializeObject(File.ReadAllText(Path.Combine(snapshot,"info.json"),Utf8));
                    if(!Equals(Get(info,"folder"),Canonical(root))) continue;
                    after=Read(snapshot);object[] oldHashes=Array(Get(info,"before")),newHashes=Array(Get(info,"after"));
                    if(oldHashes==null||newHashes==null||oldHashes.Length!=2||newHashes.Length!=2) continue;
                    if(Enumerable.Range(0,2).Any(i=>!Equals(oldHashes[i],Hash(after[i]))||!Equals(newHashes[i],Hash(before[i])))) continue;
                } catch(IOException) {continue;} catch(ArgumentException) {continue;} catch(InvalidOperationException) {continue;} catch(UnauthorizedAccessException) {continue;}
                log("[BACKUP] "+snapshot);Transaction(root,before,after);
                for(int i=0;i<2;i++) log("["+(Equal(before[i],after[i])?"SEM ALTERAÇÃO":"RESTAURADO")+"] "+Names[i]);return;
            }
            throw new IOException("Nenhum backup válido corresponde aos arquivos atuais. Eles foram editados/atualizados ou o backup está incompleto.");
        }
    }
}
