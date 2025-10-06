module Wool
  class Users
    enum Site
      Telegram
      Max
    end

    class Pseudonym
      mserializable

      getter user_id : Id
      getter site : Site
      getter name : String

      def_equals_and_hash @user_id, @site, @name

      getter id : Id { Id.from_serializable self }

      def initialize(@user_id, @site, @name)
      end
    end
  end
end
