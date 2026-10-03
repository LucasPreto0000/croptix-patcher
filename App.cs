using System;
using System.IO;
using System.Text;
using System.Diagnostics;
using System.Threading.Tasks;
using System.Windows.Forms;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Reflection;
using System.Runtime.InteropServices;

[assembly: AssemblyTitle("CrOptix Patcher")]
[assembly: AssemblyVersion("1.2.0.0")]
[assembly: AssemblyFileVersion("1.2.0.0")]

static class Theme {
    public static Color Background = Color.FromArgb(18, 20, 25);
    public static Color Card = Color.FromArgb(27, 30, 37);
    public static Color Border = Color.FromArgb(49, 54, 64);
    public static Color Accent = Color.FromArgb(255, 111, 45);
    public static Color Text = Color.FromArgb(243, 244, 247);
    public static Color Muted = Color.FromArgb(151, 158, 173);
    public static GraphicsPath Round(Rectangle rect, int radius) {
        var p = new GraphicsPath(); int d = radius * 2;
        p.AddArc(rect.X, rect.Y, d, d, 180, 90); p.AddArc(rect.Right-d, rect.Y, d, d, 270,90);
        p.AddArc(rect.Right-d, rect.Bottom-d, d,d,0,90); p.AddArc(rect.X, rect.Bottom-d,d,d,90,90); p.CloseFigure(); return p;
    }
}
class Card : Panel {
    public Card() { DoubleBuffered = true; BackColor = Theme.Card; }
    protected override void OnPaint(PaintEventArgs e) {
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        using(var p=Theme.Round(new Rectangle(0,0,Width-1,Height-1),12))
        using(var b=new SolidBrush(Theme.Card))
        using(var pen=new Pen(Theme.Border)) { e.Graphics.FillPath(b,p); e.Graphics.DrawPath(pen,p); }
        base.OnPaint(e);
    }
}
class ActionButton : Button {
    public bool Primary; bool hover;
    public ActionButton() { FlatStyle=FlatStyle.Flat; FlatAppearance.BorderSize=0; Cursor=Cursors.Hand; Font=new Font("Segoe UI",10,FontStyle.Bold); }
    protected override void OnMouseEnter(EventArgs e) { hover=true; Invalidate(); base.OnMouseEnter(e); }
    protected override void OnMouseLeave(EventArgs e) { hover=false; Invalidate(); base.OnMouseLeave(e); }
    protected override void OnPaint(PaintEventArgs e) {
        e.Graphics.Clear(Parent.BackColor); e.Graphics.SmoothingMode=SmoothingMode.AntiAlias;
        Color fill=Primary?Theme.Accent:Theme.Card;
        if(hover && Enabled) fill=Primary?Color.FromArgb(255,135,70):Color.FromArgb(43,48,58);
        if(!Enabled) fill=Color.FromArgb(51,54,61);
        using(var p=Theme.Round(new Rectangle(0,0,Width-1,Height-1),9))
        using(var b=new SolidBrush(fill))
        using(var pen=new Pen(Primary?fill:Theme.Border)) {e.Graphics.FillPath(b,p);e.Graphics.DrawPath(pen,p);}
        TextRenderer.DrawText(e.Graphics,Text,Font,ClientRectangle,Enabled?(Primary?Color.FromArgb(25,20,17):Theme.Text):Theme.Muted,TextFormatFlags.HorizontalCenter|TextFormatFlags.VerticalCenter);
        if(Focused) ControlPaint.DrawFocusRectangle(e.Graphics,new Rectangle(5,5,Width-10,Height-10));
    }
}
class Patcher {
    [DllImport("dwmapi.dll")] static extern int DwmSetWindowAttribute(IntPtr hwnd,int attribute,ref int value,int size);
    static int Run(string folder,bool restore,Action<string> log) {
        var timer=Stopwatch.StartNew();
        try {
            if(restore) PatchEngine.Restore(folder,log);else PatchEngine.Apply(folder,log);
            log("[OK] Concluído em "+timer.Elapsed.TotalSeconds.ToString("0.00")+"s. Recarregue a extensão e a página.");return 0;
        } catch(Exception error) {log("[ERRO] "+error.Message);return 1;}
    }
    static Label Label(string text,int x,int y,int w,int h,int size,bool bold,Color color) {
        return new Label {Text=text,Location=new Point(x,y),Size=new Size(w,h),ForeColor=color,BackColor=Color.Transparent,Font=new Font("Segoe UI",size,bold?FontStyle.Bold:FontStyle.Regular)};
    }
    static Form Build() {
        var form=new Form {Text="CrOptix Patcher",ClientSize=new Size(900,706),BackColor=Theme.Background,ForeColor=Theme.Text,Font=new Font("Segoe UI",10),StartPosition=FormStartPosition.CenterScreen,FormBorderStyle=FormBorderStyle.FixedSingle,MaximizeBox=false,AutoScaleMode=AutoScaleMode.Dpi};
        using(var iconStream=Assembly.GetExecutingAssembly().GetManifestResourceStream("croptix.ico"))form.Icon=new Icon(iconStream,32,32);
        form.Shown+=delegate {try{int dark=1;DwmSetWindowAttribute(form.Handle,20,ref dark,4);}catch{}};
        form.Load+=delegate {
            Rectangle work=Screen.FromControl(form).WorkingArea;
            float fit=Math.Min(1f,Math.Min((work.Width-32f)/form.Width,(work.Height-48f)/form.Height));
            if(fit<1f){Size initial=form.ClientSize;form.Scale(new SizeF(fit,fit));form.ClientSize=new Size((int)(initial.Width*fit),(int)(initial.Height*fit));}
            form.Location=new Point(work.Left+(work.Width-form.Width)/2,work.Top+(work.Height-form.Height)/2);
        };
        form.Controls.Add(Label("C",30,22,48,52,30,true,Theme.Accent));
        form.Controls.Add(Label("CrOptix Patcher",88,23,490,37,23,true,Theme.Text));
        form.Controls.Add(Label("Seu player, com os ajustes que fazem diferença.",90,64,660,26,10,false,Theme.Muted));
        form.Controls.Add(Label("PATCH INDEPENDENTE",664,34,209,24,9,true,Theme.Accent));
        var folderCard=new Card {Location=new Point(30,112),Size=new Size(840,108)};
        folderCard.Controls.Add(Label("Pasta da extensão",18,13,250,25,11,true,Theme.Text));
        folderCard.Controls.Add(Label("Selecione a pasta com katamari.js e manifest.json.",18,39,625,22,9,false,Theme.Muted));
        var pathBox=new TextBox {Location=new Point(19,69),Size=new Size(657,24),BorderStyle=BorderStyle.None,BackColor=Theme.Card,ForeColor=Theme.Text,Font=new Font("Segoe UI",10)};
        pathBox.Text="";folderCard.Controls.Add(pathBox);
        var browse=new ActionButton {Text="Selecionar",Location=new Point(691,60),Size=new Size(132,34)};
        folderCard.Controls.Add(browse);form.Controls.Add(folderCard);
        form.Controls.Add(Label("CORREÇÕES INCLUÍDAS",30,240,420,22,9,true,Theme.Muted));
        string[] titles={"PiP com legendas","Próximo episódio","Avanço sem espera","Controles discretos"};
        string[] details={"Vídeo e legendas na mesma janela.","Botão alinhado ao lado do volume.","Setas e J/L respondem a cada repetição.","A barra fica oculta ao avançar."};
        for(int i=0;i<4;i++){
            var card=new Card {Location=new Point(30+(i%2)*428,272+(i/2)*73),Size=new Size(412,63)};
            card.Controls.Add(Label("✓",16,18,29,29,16,true,Theme.Accent));
            card.Controls.Add(Label(titles[i],52,10,345,24,11,true,Theme.Text));
            card.Controls.Add(Label(details[i],52,34,345,22,9,false,Theme.Muted));form.Controls.Add(card);
        }
        form.Controls.Add(Label("RELATÓRIO",30,424,250,22,9,true,Theme.Muted));
        var log=new RichTextBox {Location=new Point(30,455),Size=new Size(840,153),BackColor=Color.FromArgb(13,15,19),ForeColor=Theme.Muted,BorderStyle=BorderStyle.None,ReadOnly=true,DetectUrls=false,ScrollBars=RichTextBoxScrollBars.None,Font=new Font("Consolas",10),Text="Selecione a extensão para começar.\n\n• Validação antes de modificar arquivos\n• Backup automático\n• Detecção de correções já aplicadas"};
        form.Controls.Add(log);
        var status=Label("Pronto para selecionar uma pasta.",30,636,420,25,10,false,Theme.Muted);
        var restore=new ActionButton {Text="Restaurar backup",Location=new Point(481,628),Size=new Size(178,45)};
        var apply=new ActionButton {Text="Aplicar patch",Primary=true,Location=new Point(675,628),Size=new Size(195,45)};
        form.Controls.Add(status);form.Controls.Add(restore);form.Controls.Add(apply);
        bool running=false;
        form.FormClosing+=delegate(object sender,FormClosingEventArgs e){if(running)e.Cancel=true;};
        browse.Click+=delegate {
            try {string selected=ExplorerFolderPicker.Select(form.Handle,pathBox.Text);if(selected!=null){pathBox.Text=selected;status.Text="Pasta selecionada.";}}
            catch(Exception error){log.Text="Não foi possível abrir o seletor do Explorador.\n"+error.Message;}
        };
        Func<bool,Task> execute=async undo=>{
            string folder=pathBox.Text.Trim().Trim('"');
            if(!File.Exists(Path.Combine(folder,"katamari.js"))||!File.Exists(Path.Combine(folder,"manifest.json"))){log.ForeColor=Color.FromArgb(255,145,125);log.Text="Pasta inválida.\nSelecione uma pasta com katamari.js e manifest.json.\nNenhum arquivo foi alterado.";return;}
            if(undo&&MessageBox.Show(form,"Restaurar os arquivos do backup mais recente desta pasta?","Restaurar backup",MessageBoxButtons.YesNo,MessageBoxIcon.Question)!=DialogResult.Yes)return;
            running=true;apply.Enabled=restore.Enabled=browse.Enabled=pathBox.Enabled=false;
            status.Text=undo?"Restaurando backup…":"Aplicando correções…";log.ForeColor=Theme.Muted;log.Text="Validando arquivos…";
            try {
                log.Clear();
                Action<string> append=line=>form.Invoke((Action)(()=>{log.AppendText(line+Environment.NewLine);log.SelectionStart=log.TextLength;log.ScrollToCaret();}));
                int exit=await Task.Run(()=>Run(folder,undo,append));
                log.ForeColor=exit==0?Theme.Text:Color.FromArgb(255,145,125);status.Text=exit==0?"Concluído. Recarregue a extensão.":"Não aplicado. Confira o relatório.";
            }
            catch(Exception error){log.ForeColor=Color.FromArgb(255,145,125);log.Text=error.Message;status.Text="Falha ao executar.";}
            finally{running=false;apply.Enabled=restore.Enabled=browse.Enabled=pathBox.Enabled=true;}
        };
        apply.Click+=async delegate{await execute(false);};restore.Click+=async delegate{await execute(true);};
        return form;
    }
    [STAThread] static int Main(string[] args) {
        if(args.Length==2&&args[0]=="--picker-self-test"){try{return ExplorerFolderPicker.SelfTest(args[1])?0:1;}catch{return 1;}}
        if(args.Length==2&&(args[0]=="--apply"||args[0]=="--restore")){
            return Run(args[1],args[0]=="--restore",Console.WriteLine);
        }
        if(args.Length!=0) return 2;
        Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);
        using(var form=Build()){
            Application.Run(form);
        }return 0;
    }
}
