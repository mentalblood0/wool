module Wool
  class Users
    enum Site
      Telegram
      Max
    end

    class Integration
      mserializable

      getter user_id : Id
      getter site : Site
      getter pseudonym : String

      def_equals_and_hash @user_id, @site, @pseudonym

      getter id : Id { Id.from_serializable self }

      def initialize(@user_id, @site, @pseudonym)
      end
    end
  end
end
