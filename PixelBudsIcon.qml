import QtQuick
import qs.Commons

// Drawn rather than shipped as an SVG, same call omapods made for AirPods: a
// two-part silhouette loses its detail to rasterisation at bar size. Pixel
// Buds Pro are stemless, so this is one rounded body per bud rather than a
// head-and-stem pair.
Item {
  id: root

  property real iconSize: 16
  property color color: Color.foreground

  implicitWidth: iconSize
  implicitHeight: iconSize

  readonly property real budWidth: iconSize * 0.4
  readonly property real budHeight: iconSize * 0.62

  Row {
    anchors.centerIn: parent
    spacing: root.iconSize * 0.14

    Bud { w: root.budWidth; h: root.budHeight; ink: root.color }
    Bud { w: root.budWidth; h: root.budHeight; ink: root.color }
  }

  component Bud: Item {
    id: bud
    property real w: 0
    property real h: 0
    property color ink: Color.foreground

    width: w
    height: h

    // The rounded body.
    Rectangle {
      anchors.fill: parent
      radius: width / 2.1
      color: bud.ink
    }

    // The ear-tip nub, a shade darker so the silhouette still reads as an
    // earbud rather than a plain capsule.
    Rectangle {
      anchors.horizontalCenter: parent.horizontalCenter
      y: bud.h * 0.72
      width: bud.w * 0.42
      height: bud.h * 0.3
      radius: width / 2
      color: Qt.darker(bud.ink, 1.6)
    }
  }
}
