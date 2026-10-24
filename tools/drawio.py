"""Tiny UML 2 draw.io generator. Each arch/src/<Name>.py builds pages and writes arch/<Name>.drawio.
Usage: python3 arch/src/<Name>.py  (run from repo root)"""
from xml.sax.saxutils import escape as _e
import xml.dom.minidom

NODE = "shape=cube;whiteSpace=wrap;html=1;boundedLbl=1;size=12;verticalAlign=top;align=left;spacingLeft=8;fontStyle=1;"
COMP = "shape=component;align=left;spacingLeft=36;html=1;whiteSpace=wrap;"
EXT = COMP + "dashed=1;fillColor=#f5f5f5;"
ART = "shape=note;size=12;html=1;whiteSpace=wrap;"
FRAME = "shape=umlFrame;whiteSpace=wrap;html=1;width=260;height=24;verticalAlign=top;align=left;spacingLeft=8;"
TITLE = "text;fontSize=16;fontStyle=1;html=1;"
NOTE = "shape=note;size=14;html=1;whiteSpace=wrap;align=left;spacingLeft=6;fillColor=#fff2cc;"


class Page:
    _seq = [1]

    def __init__(self, pid, name, w=1400, h=900):
        self.pid, self.name, self.w, self.h, self.cells = pid, name, w, h, []

    def _id(self):
        Page._seq[0] += 1
        return f"c{Page._seq[0]}"

    def box(self, label, x, y, w, h, style):
        i = self._id()
        self.cells.append(f'<mxCell id="{i}" value="{_e(label)}" style="{style}" vertex="1" parent="1">'
                          f'<mxGeometry x="{x}" y="{y}" width="{w}" height="{h}" as="geometry"/></mxCell>')
        return i

    def edge(self, s, t, label="", dashed=True):
        st = "endArrow=open;endSize=12;html=1;" + ("dashed=1;" if dashed else "")
        self.cells.append(f'<mxCell id="{self._id()}" value="{_e(label)}" style="{st}" edge="1" parent="1" source="{s}" target="{t}">'
                          '<mxGeometry relative="1" as="geometry"/></mxCell>')

    def xml(self):
        return (f'<diagram id="{self.pid}" name="{_e(self.name, {chr(34): "&quot;"})}"><mxGraphModel dx="{self.w}" dy="{self.h}" grid="1" gridSize="10" page="1" '
                f'pageWidth="{self.w}" pageHeight="{self.h}"><root><mxCell id="0"/><mxCell id="1" parent="0"/>{"".join(self.cells)}</root></mxGraphModel></diagram>')


def write(path, pages):
    out = '<mxfile host="drawio" version="24.0.0">' + "".join(p.xml() for p in pages) + "</mxfile>"
    xml.dom.minidom.parseString(out)  # validate
    open(path, "w").write(out)
    print(f"wrote {path} ({len(pages)} pages)")


CLS = "verticalAlign=top;align=left;overflow=fill;html=1;whiteSpace=wrap;fontSize=11;"
ENUM = CLS + "fillColor=#f5f5f5;"
PKG = "shape=folder;fontStyle=1;tabWidth=140;tabHeight=22;tabPosition=left;html=1;whiteSpace=wrap;verticalAlign=top;align=left;spacingLeft=8;spacingTop=2;fillColor=none;"


def cls_label(name, attrs=(), stereo=None):
    """UML class box as HTML: name compartment + attribute compartment."""
    head = (f"<i>«{stereo}»</i><br/>" if stereo else "") + f"<b>{name}</b>"
    body = "<br/>".join(a.replace("<", "&lt;").replace(">", "&gt;") for a in attrs)
    return f'<p style="margin:4px;text-align:center">{head}</p><hr size="1"/><p style="margin:0 6px">{body}</p>'


def _raw_box(page, html, x, y, w, h, style):
    i = page._id()
    from html import escape
    page.cells.append(f'<mxCell id="{i}" value="{escape(html, quote=True)}" style="{style}" vertex="1" parent="1">'
                      f'<mxGeometry x="{x}" y="{y}" width="{w}" height="{h}" as="geometry"/></mxCell>')
    return i


def cls(page, name, attrs, x, y, w=200, stereo=None, style=CLS):
    h = 34 + 15 * max(1, len(attrs)) + (14 if stereo else 0)
    return _raw_box(page, cls_label(name, attrs, stereo), x, y, w, h, style)


def assoc(page, s, t, src_mult="", tgt_mult="", label="", kind="assoc"):
    """kind: assoc | composition | aggregation | generalization | dependency"""
    styles = {
        "assoc": "endArrow=none;html=1;",
        "composition": "startArrow=diamondThin;startFill=1;startSize=14;endArrow=none;html=1;",
        "aggregation": "startArrow=diamondThin;startFill=0;startSize=14;endArrow=none;html=1;",
        "generalization": "endArrow=block;endFill=0;endSize=12;html=1;",
        "dependency": "endArrow=open;endSize=12;dashed=1;html=1;",
    }
    lab = label
    if src_mult or tgt_mult:
        lab = f"{src_mult} ── {label + ' ── ' if label else ''}{tgt_mult}".strip()
    page.cells.append(f'<mxCell id="{page._id()}" value="{_e(lab)}" style="{styles[kind]}edgeStyle=orthogonalEdgeStyle;rounded=0;fontSize=10;" edge="1" parent="1" source="{s}" target="{t}">'
                      '<mxGeometry relative="1" as="geometry"/></mxCell>')
