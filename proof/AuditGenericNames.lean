import QCore.DescriptorCustody
import QCore.MonkeyEvent
import QCore.ExternalIntegration
import QCore.ChunkSource

-- The legacy namespace resolves the same types, constructors and checked functions.
example (pair : QCore.MonkeyEvent.SocketPair) :
    QCore.MonkeyEvent.checkSocketPair pair =
      QCore.DescriptorCustody.checkSocketPair pair := rfl

example (pair : QCore.MonkeyEvent.SocketPair) :
    pair.Valid = QCore.DescriptorCustody.SocketPair.Valid pair := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    composition.Valid = QCore.ExternalIntegration.EventComposition.Valid composition := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    composition.buildProfileAccepted =
      QCore.ExternalIntegration.EventComposition.buildProfileAccepted composition := rfl

example (composition : QCore.ExternalIntegration.MonkeyComposition) :
    QCore.ExternalIntegration.checkMonkeyComposition composition =
      QCore.ExternalIntegration.checkEventComposition composition := rfl

example (certificate : QCore.ExternalIntegration.MonkeyCompositionCertificate) :
    certificate.checked =
      QCore.ExternalIntegration.EventCompositionCertificate.checked certificate := rfl

#print axioms QCore.DescriptorCustody.accepted_descriptor_teardown_is_complete_and_unique
#print axioms QCore.MonkeyEvent.accepted_monkey_descriptor_teardown_is_complete_and_unique
#print axioms QCore.ChunkSource.busy_effect_preserves_lock
#print axioms QCore.ChunkSource.busy_effect_preserves_chunkio_lock
#print axioms QCore.ExternalIntegration.accepted_monkey_composition_has_event_teardown
