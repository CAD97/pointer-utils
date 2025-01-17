(function() {
    var implementors = Object.fromEntries([["erasable",[["impl&lt;P&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> for <a class=\"struct\" href=\"erasable/struct.Thin.html\" title=\"struct erasable::Thin\">Thin</a>&lt;P&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> + <a class=\"trait\" href=\"erasable/trait.ErasablePtr.html\" title=\"trait erasable::ErasablePtr\">ErasablePtr</a>,</div>"]]],["rc_box",[["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> for <a class=\"struct\" href=\"rc_box/struct.ArcBox.html\" title=\"struct rc_box::ArcBox\">ArcBox</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div>"],["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> for <a class=\"struct\" href=\"rc_box/struct.RcBox.html\" title=\"struct rc_box::RcBox\">RcBox</a>&lt;T&gt;<div class=\"where\">where\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a> + ?<a class=\"trait\" href=\"https://doc.rust-lang.org/beta/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</div>"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[564,1156]}